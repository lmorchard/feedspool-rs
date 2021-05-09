#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::large_enum_variant)]

use chrono::prelude::*;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use feed_rs::model::Entry;
use feed_rs::parser;
use sha2::{Digest, Sha256};
use std::time::Duration;

mod db;
pub mod result;

use db::{
    feed_id_from_url, find_last_fetch_time, find_last_get_conditions, insert_feed_history,
    upsert_entry, upsert_feed,
};
use result::{ConditionalGetData, FeedFetchResult, FeedPollError, FeedPollResult};

/// # Errors
///
/// Will return Err for any failure while polling a feed
pub async fn poll_one_feed(
    conn: &SqliteConnection,
    url: &str,
    request_timeout: Duration,
    min_fetch_period: Duration,
) -> Result<FeedPollResult, FeedPollError> {
    match _poll_one_feed(conn, url, request_timeout, min_fetch_period).await {
        Ok(fetch_result) => {
            record_feed_history_success(&conn, &fetch_result)?;
            Ok(fetch_result)
        }
        Err(error) => {
            record_feed_history_error(&conn, &url, &error)?;
            Err(error)
        }
    }
}

async fn _poll_one_feed(
    conn: &SqliteConnection,
    url: &str,
    request_timeout: Duration,
    min_fetch_period: Duration,
) -> Result<FeedPollResult, FeedPollError> {
    if was_feed_recently_fetched(&conn, &url, min_fetch_period)? {
        log::trace!("Skipped fetch for {} - min fetch period", &url);
        return Ok(FeedPollResult::Skipped);
    }
    let last_get_conditions = find_last_get_conditions(&conn, &url);
    let mut fetch_result = fetch_feed(url, request_timeout, last_get_conditions).await?;
    fetch_result = update_feed(&conn, fetch_result)?;
    Ok(fetch_result)
}

fn was_feed_recently_fetched(
    conn: &SqliteConnection,
    url: &str,
    min_fetch_period: Duration,
) -> Result<bool, FeedPollError> {
    let now = Utc::now();
    let min_fetch_duration = match chrono::Duration::from_std(min_fetch_period) {
        Err(error) => Err(FeedPollError::FetchTimeError(error)),
        Ok(val) => Ok(val),
    }?;
    let last_fetch_time = find_last_fetch_time(&conn, &url);
    if let Some(last_fetch_time) = last_fetch_time {
        if let Ok(last_fetch_time) = chrono::DateTime::parse_from_rfc3339(&last_fetch_time) {
            if now < last_fetch_time + min_fetch_duration {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

async fn fetch_feed(
    url: &str,
    timeout_duration: Duration,
    last_get_conditions: Option<ConditionalGetData>,
) -> Result<FeedPollResult, FeedPollError> {
    let mut request = reqwest::Client::new().get(url).timeout(timeout_duration);
    if let Some(last_get_conditions) = last_get_conditions {
        if let Some(etag) = last_get_conditions.etag {
            request = request.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        if let Some(last_modified) = last_get_conditions.last_modified {
            request = request.header(reqwest::header::IF_MODIFIED_SINCE, last_modified);
        }
    }

    match request.send().await {
        Err(error) => {
            if error.is_timeout() {
                Err(FeedPollError::Timedout(error))
            } else if error.is_status() && error.status().unwrap() == reqwest::StatusCode::NOT_FOUND
            {
                Err(FeedPollError::NotFound(error))
            } else {
                Err(FeedPollError::FetchError(error))
            }
        }
        Ok(response) => {
            let response_status = response.status();
            let headers = response.headers().clone();
            let body = response.text().await;
            match body {
                Err(error) => {
                    if error.is_timeout() {
                        Err(FeedPollError::Timedout(error))
                    } else {
                        Err(FeedPollError::FetchError(error))
                    }
                }
                Ok(body) => {
                    let fetch = FeedFetchResult {
                        id: feed_id_from_url(&url),
                        url: String::from(url),
                        status: String::from(response_status.as_str()),
                        headers,
                        body,
                    };
                    match response_status {
                        reqwest::StatusCode::OK => match parser::parse(fetch.body.as_bytes()) {
                            Err(error) => Err(FeedPollError::ParseError { fetch, error }),
                            Ok(feed) => Ok(FeedPollResult::Fetched { fetch, feed }),
                        },
                        reqwest::StatusCode::NOT_MODIFIED => {
                            Ok(FeedPollResult::NotModified { fetch })
                        }
                        _ => Err(FeedPollError::FetchFailed { fetch }),
                    }
                }
            }
        }
    }
}

fn update_feed(
    conn: &SqliteConnection,
    fetch_result: FeedPollResult,
) -> Result<FeedPollResult, FeedPollError> {
    match &fetch_result {
        FeedPollResult::Fetched { feed, fetch, .. } => {
            let now = Utc::now();

            let mut feed_published = String::from(&now.to_rfc3339());
            if let Some(published_date) = feed.published {
                if published_date < now {
                    feed_published = published_date.to_rfc3339();
                }
            };

            let mut feed_title = "";
            if let Some(t) = &feed.title {
                feed_title = &t.content;
            }

            let mut feed_link = "";
            if !feed.links.is_empty() {
                // TODO: handle multiple links?
                feed_link = &feed.links[0].href;
            }

            if let Err(error) = upsert_feed(
                &conn,
                &now,
                &fetch.id,
                &fetch.url,
                &feed_title,
                &feed_link,
                &feed_published,
            ) {
                return Err(fetch_result.fetched_to_update_error(error));
            }
            for entry in &feed.entries {
                if let Err(error) = update_entry(&conn, &fetch.id, &entry) {
                    return Err(fetch_result.fetched_to_update_error(error));
                }
            }

            Ok(fetch_result.fetched_to_updated())
        }
        _ => Ok(fetch_result),
    }
}

fn record_feed_history_success(
    conn: &SqliteConnection,
    fetch_result: &FeedPollResult,
) -> Result<(), FeedPollError> {
    match &fetch_result {
        FeedPollResult::Updated { fetch, .. } | FeedPollResult::NotModified { fetch, .. } => {
            insert_feed_history(&conn, &fetch)?;
            Ok(())
        }
        _ => Ok(()),
    }
}

fn record_feed_history_error(
    conn: &SqliteConnection,
    url: &str,
    error: &FeedPollError,
) -> Result<(), FeedPollError> {
    let now = Utc::now().to_rfc3339();
    let feed_id = feed_id_from_url(&url);
    let history_id = &format!("{:x}", Sha256::new().chain(&feed_id).chain(&now).finalize());
    {
        use crate::models;
        use crate::schema::feed_history;
        if let Err(db_error) = diesel::insert_into(feed_history::table)
            .values(models::FeedHistoryNewError {
                id: history_id,
                feed_id: &feed_id,
                created_at: &now,
                is_error: true,
                error_text: format!("{:?}", &error).as_str(),
            })
            .execute(conn)
        {
            return Err(FeedPollError::DatabaseError(db_error));
        }
    }
    Ok(())
}

fn update_entry(
    conn: &SqliteConnection,
    parent_feed_id: &str,
    entry: &Entry,
) -> Result<(), diesel::result::Error> {
    let now = Utc::now();

    let mut entry_published = String::from(&now.to_rfc3339());
    if let Some(published_date) = entry.published {
        if published_date < now {
            entry_published = published_date.to_rfc3339();
        }
    };

    let mut entry_title = "";
    if let Some(t) = &entry.title {
        entry_title = &t.content;
    }

    let mut entry_link = "";
    if !entry.links.is_empty() {
        entry_link = &entry.links[0].href;
    }

    let mut entry_summary = "";
    if let Some(x) = &entry.summary {
        entry_summary = &x.content;
    }

    let mut entry_content = "";
    if let Some(e_content) = &entry.content {
        if let Some(e_body) = &e_content.body {
            entry_content = e_body.as_ref();
        }
    }

    let entry_id = format!(
        "{:x}",
        Sha256::new()
            .chain(&entry.id)
            .chain(&entry_title)
            .chain(&entry_link)
            .finalize()
    );

    upsert_entry(
        &conn,
        &now,
        parent_feed_id,
        &entry_id,
        &entry_published,
        &entry_title,
        &entry_link,
        &entry_summary,
        &entry_content,
    )?;
    Ok(())
}
