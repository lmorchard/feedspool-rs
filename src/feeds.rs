#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::large_enum_variant)]

use chrono::prelude::*;
use diesel::sqlite::SqliteConnection;
use feed_rs::model::Entry;
use feed_rs::parser;
use sha2::{Digest, Sha256};
use std::time::Duration;

pub mod result;

use crate::db::{
    feed_id_from_url, find_last_fetch_time, find_last_get_conditions, insert_feed_history,
    insert_feed_history_error, upsert_entry, upsert_feed,
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
    retain_src: bool,
) -> Result<FeedPollResult, FeedPollError> {
    // TODO: this wraps another function so I can try/catch an Err() from any of the ? operators - is there a better way?
    match _poll_one_feed(conn, url, request_timeout, min_fetch_period).await {
        Ok(fetch_result) => {
            if let FeedPollResult::Updated { fetch, .. }
            | FeedPollResult::NotModified { fetch, .. } = &fetch_result
            {
                insert_feed_history(&conn, &fetch, retain_src)?;
            }
            Ok(fetch_result)
        }
        Err(error) => {
            insert_feed_history_error(&conn, &url, &error)?;
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

/// # Errors
///
/// Will return `FeedPollError` for any failure while fetching a feed
///
/// # Panics
///
/// Shouldn't be any panics here
pub async fn fetch_feed(
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

fn clamp_future_date_to_now<'a>(
    now: &'a DateTime<Utc>,
    thedate: &'a DateTime<Utc>,
) -> &'a DateTime<Utc> {
    if thedate < now {
        return thedate;
    }
    now
}

fn update_feed(
    conn: &SqliteConnection,
    fetch_result: FeedPollResult,
) -> Result<FeedPollResult, FeedPollError> {
    use crate::models;

    if let FeedPollResult::Fetched { feed, fetch, .. } = &fetch_result {
        let now = Utc::now();

        let mut last_entry_published: Option<DateTime<Utc>> = None;
        for entry in &feed.entries {
            if let Err(error) = update_entry(&conn, &fetch.id, &entry) {
                return Err(fetch_result.fetched_to_update_error(error));
            }
            if let Some(entry_published) = &entry.published {
                let entry_published = clamp_future_date_to_now(&now, entry_published);
                if last_entry_published.is_none()
                    || entry_published > last_entry_published.as_ref().unwrap()
                {
                    last_entry_published.replace(*entry_published);
                }
            }
        }

        match upsert_feed(
            &conn,
            &models::FeedUpsert {
                now: &now.to_rfc3339(),
                id: &fetch.id,
                url: &fetch.url,
                json: &serde_json::to_string(&feed).unwrap_or_else(|_| String::from("")),
                last_entry_published: &last_entry_published
                    .map_or_else(|| String::from(""), |dt| dt.to_rfc3339()),
                published: &feed.published.map_or(String::from(""), |dt| {
                    clamp_future_date_to_now(&now, &dt).to_rfc3339()
                }),
                updated: &feed.updated.map_or(String::from(""), |dt| {
                    clamp_future_date_to_now(&now, &dt).to_rfc3339()
                }),
                title: &feed
                    .title
                    .as_ref()
                    .map_or_else(|| String::from(""), |title| String::from(&title.content)),
                link: &feed
                    .links
                    .first()
                    .map_or_else(|| String::from(""), |link| String::from(&link.href)),
            },
        ) {
            Err(error) => Err(fetch_result.fetched_to_update_error(error)),
            Ok(_) => Ok(fetch_result.fetched_to_updated()),
        }
    } else {
        Ok(fetch_result)
    }
}

fn update_entry(
    conn: &SqliteConnection,
    parent_feed_id: &str,
    entry: &Entry,
) -> Result<(), diesel::result::Error> {
    use crate::models;
    let now = Utc::now();

    upsert_entry(
        &conn,
        &models::EntryUpsert {
            // TODO: make this update skip optional?
            skip_update: true,
            now: &now.to_rfc3339(),
            id: &format!(
                "{:x}",
                Sha256::new()
                    .chain(&parent_feed_id)
                    .chain(&entry.id)
                    .finalize()
            ),
            feed_id: &parent_feed_id,
            json: &serde_json::to_string(&entry).unwrap_or_else(|_| String::from("")),
            published: &entry.published.map_or(String::from(""), |dt| {
                clamp_future_date_to_now(&now, &dt).to_rfc3339()
            }),
            updated: &entry.updated.map_or(String::from(""), |dt| {
                clamp_future_date_to_now(&now, &dt).to_rfc3339()
            }),
            title: &entry
                .title
                .as_ref()
                .map_or_else(|| String::from(""), |title| String::from(&title.content)),
            link: &entry
                .links
                .first()
                .map_or_else(|| String::from(""), |link| String::from(&link.href)),
            summary: &entry.summary.as_ref().map_or_else(
                || String::from(""),
                |summary| String::from(&summary.content),
            ),
            content: &entry.content.as_ref().map_or_else(
                || String::from(""),
                |content| {
                    content
                        .body
                        .as_ref()
                        .map_or_else(|| String::from(""), ToString::to_string)
                },
            ),
        },
    )?;
    Ok(())
}
