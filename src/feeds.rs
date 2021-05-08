#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::large_enum_variant)]

use chrono::prelude::*;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use feed_rs::model::{Entry, Feed};
use feed_rs::parser;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;
use std::panic;
use std::time::Duration;

#[derive(Debug)]
pub enum FeedPollResult {
    Skipped {
        id: String,
        url: String,
    },
    Fetched {
        id: String,
        url: String,
        status: String,
        body: String,
        headers: reqwest::header::HeaderMap,
        feed: Feed,
    },
    NotModified {
        id: String,
        url: String,
        status: String,
        body: String,
        headers: reqwest::header::HeaderMap,
    },
    Updated {
        id: String,
        url: String,
        status: String,
        body: String,
        headers: reqwest::header::HeaderMap,
        feed: Feed,
    },
}

impl FeedPollResult {
    #[must_use]
    pub fn skipped(url: &str) -> FeedPollResult {
        Self::Skipped {
            id: feed_id_from_url(&url),
            url: String::from(url),
        }
    }

    /// # Panics
    /// 
    /// Will panic if attempted on something other than `FeedPollResult::Fetched`
    #[must_use]
    pub fn fetched_to_updated(self) -> FeedPollResult {
        match self {
            Self::Fetched {
                id,
                url,
                status,
                body,
                headers,
                feed,
            } => Self::Updated {
                id,
                url,
                status,
                body,
                headers,
                feed,
            },
            // TODO: find a non-panic way to handle this?
            _ => panic!("fetched_to_updated {:?}", &self),
        }
    }

    /// # Panics
    /// 
    /// Will panic if attempted on something other than `FeedPollResult::Fetched`
    #[must_use]
    pub fn fetched_to_update_error(self, error: diesel::result::Error) -> FeedPollError {
        match self {
            Self::Fetched {
                id,
                url,
                status,
                body,
                headers,
                feed,
            } => FeedPollError::UpdateError {
                id,
                url,
                status,
                body,
                headers,
                feed,
                error,
            },
            // TODO: find a non-panic way to handle this?
            _ => panic!("fetched_to_update_error {:?}", self),
        }
    }
}

#[derive(Debug)]
pub enum FeedPollError {
    FetchTimeError(time::OutOfRangeError),
    Timedout {
        id: String,
        url: String,
        error: reqwest::Error,
    },
    NotFound {
        id: String,
        url: String,
        error: reqwest::Error,
    },
    FetchError {
        id: String,
        url: String,
        error: reqwest::Error,
    },
    FetchFailed {
        id: String,
        url: String,
        status: String,
        body: String,
        headers: reqwest::header::HeaderMap,
    },
    ParseError {
        id: String,
        url: String,
        status: String,
        body: String,
        headers: reqwest::header::HeaderMap,
        error: feed_rs::parser::ParseFeedError,
    },
    UpdateError {
        id: String,
        url: String,
        status: String,
        body: String,
        headers: reqwest::header::HeaderMap,
        feed: Feed,
        error: diesel::result::Error,
    },
}
impl FeedPollError {}
impl fmt::Display for FeedPollError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: better formatting for these errors
        write!(f, "{:?}", &self)
    }
}
impl Error for FeedPollError {}

/// # Errors
///
/// Will return Err for any failure while polling a feed
/// TODO: actually inventory and document the errors here
pub async fn poll_one_feed(
    conn: &SqliteConnection,
    url: &str,
    request_timeout: Duration,
    min_fetch_period: Duration,
) -> Result<FeedPollResult, FeedPollError> {
    if was_feed_recently_fetched(&conn, &url, min_fetch_period)? {
        log::trace!("Skipped fetch for {} - min fetch period", &url);
        return Ok(FeedPollResult::skipped(&url));
    }
    let last_get_conditions = find_last_get_conditions(&conn, &url);
    // TODO: Catch error from fetch and record in history before returning
    let mut fetch_result = fetch_feed(url, request_timeout, last_get_conditions).await?;
    fetch_result = update_feed(&conn, fetch_result)?;
    fetch_result = record_feed_history(&conn, fetch_result)?;
    Ok(fetch_result)
}

fn feed_id_from_url(url: &str) -> String {
    format!("{:x}", Sha256::new().chain(url).finalize())
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

    let id = feed_id_from_url(&url);
    let url = String::from(url);

    // TODO: impl4ement some convenience constructors to shorten up some of these results
    match request.send().await {
        Err(error) => {
            if error.is_timeout() {
                Err(FeedPollError::Timedout { id, url, error })
            } else if error.is_status() && error.status().unwrap() == reqwest::StatusCode::NOT_FOUND
            {
                Err(FeedPollError::NotFound { id, url, error })
            } else {
                Err(FeedPollError::FetchError { id, url, error })
            }
        }
        Ok(response) => {
            let response_status = response.status();
            let status = String::from(response_status.as_str());
            let headers = response.headers().clone();
            match response.text().await {
                Err(error) => {
                    if error.is_timeout() {
                        Err(FeedPollError::Timedout { id, url, error })
                    } else {
                        Err(FeedPollError::FetchError { id, url, error })
                    }
                }
                Ok(body) => match response_status {
                    reqwest::StatusCode::OK => match parser::parse(body.as_bytes()) {
                        Err(error) => Err(FeedPollError::ParseError {
                            id,
                            url,
                            status,
                            headers,
                            body,
                            error,
                        }),
                        Ok(feed) => Ok(FeedPollResult::Fetched {
                            feed,
                            id,
                            url,
                            status,
                            headers,
                            body,
                        }),
                    },
                    reqwest::StatusCode::NOT_MODIFIED => Ok(FeedPollResult::NotModified {
                        id,
                        url,
                        status,
                        headers,
                        body,
                    }),
                    _ => Err(FeedPollError::FetchFailed {
                        id,
                        url,
                        status,
                        headers,
                        body,
                    }),
                },
            }
        }
    }
}

fn update_feed(
    conn: &SqliteConnection,
    fetch_result: FeedPollResult,
) -> Result<FeedPollResult, FeedPollError> {
    match &fetch_result {
        FeedPollResult::Fetched {
            feed,
            id: feed_id,
            url: feed_url,
            ..
        } => {
            let now = Utc::now();

            let feed_url = feed_url.as_str();

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
                &feed_id,
                &feed_url,
                &feed_title,
                &feed_link,
                &feed_published,
            ) {
                return Err(fetch_result.fetched_to_update_error(error));
            }
            for entry in &feed.entries {
                if let Err(error) = update_entry(&conn, &feed_id, &entry) {
                    return Err(fetch_result.fetched_to_update_error(error));
                }
            }

            Ok(fetch_result.fetched_to_updated())
        }
        _ => Ok(fetch_result),
    }
}

fn upsert_feed(
    conn: &SqliteConnection,
    now: &DateTime<Utc>,
    feed_id: &str,
    feed_url: &str,
    feed_title: &str,
    feed_link: &str,
    feed_published: &str,
) -> Result<(), diesel::result::Error> {
    use crate::models;
    use crate::schema::feeds::dsl::{feeds, id};

    let feed_exists = feeds
        .filter(id.eq(&feed_id))
        .count()
        .get_result::<i64>(conn)?
        > 0;

    if feed_exists {
        log::trace!("Feed exists {}", feed_id);
        diesel::update(feeds)
            .filter(id.eq(&feed_id))
            .set(models::FeedUpdate {
                title: Some(&feed_title),
                link: Some(&feed_link),
                url: Some(feed_url),
                published: Some(&feed_published),
                updated_at: Some(&now.to_rfc3339()),
            })
            .execute(conn)?;
    } else {
        log::trace!("Feed new {}", feed_id);
        diesel::insert_into(feeds)
            .values(models::FeedNew {
                id: &feed_id,
                title: &feed_title,
                link: &feed_link,
                url: feed_url,
                published: &feed_published,
                created_at: &now.to_rfc3339(),
                updated_at: &now.to_rfc3339(),
            })
            .execute(conn)?;
    }
    Ok(())
}

fn header_or_blank(
    headers: &reqwest::header::HeaderMap,
    name: reqwest::header::HeaderName,
) -> &str {
    if let Some(value) = &headers.get(name) {
        if let Ok(value_str) = value.to_str() {
            return value_str;
        }
    }
    ""
}

fn record_feed_history(
    conn: &SqliteConnection,
    fetch_result: FeedPollResult,
) -> Result<FeedPollResult, FeedPollError> {
    match &fetch_result {
        FeedPollResult::Fetched {
            id,
            status,
            headers,
            body,
            ..
        }
        | FeedPollResult::Updated {
            id,
            status,
            headers,
            body,
            ..
        } => {
            if let Err(error) = insert_feed_history(&conn, &id, &status, &headers, &body) {
                return Err(fetch_result.fetched_to_update_error(error));
            }
            Ok(fetch_result)
        }
        _ => Ok(fetch_result),
    }
}

fn insert_feed_history(
    conn: &SqliteConnection,
    feed_id: &str,
    status: &str,
    headers: &reqwest::header::HeaderMap,
    body: &str,
) -> Result<(), diesel::result::Error> {
    let now = Utc::now().to_rfc3339();
    let history_id = &format!("{:x}", Sha256::new().chain(&feed_id).chain(&now).finalize());
    {
        use crate::models;
        use crate::schema::feed_history;
        diesel::insert_into(feed_history::table)
            .values(models::FeedHistoryNew {
                feed_id,
                id: history_id,
                src: &body,
                status: &status,
                etag: header_or_blank(&headers, reqwest::header::ETAG),
                last_modified: header_or_blank(&headers, reqwest::header::LAST_MODIFIED),
                created_at: &now,
            })
            .execute(conn)?;
    }
    Ok(())
}

#[derive(Debug)]
struct ConditionalGetData {
    etag: Option<String>,
    last_modified: Option<String>,
}

fn find_last_get_conditions(conn: &SqliteConnection, feed_url: &str) -> Option<ConditionalGetData> {
    use crate::schema::feed_history;
    let feed_id = feed_id_from_url(feed_url);
    match feed_history::table
        .filter(feed_history::dsl::feed_id.eq(feed_id))
        .filter(feed_history::dsl::status.eq("200"))
        .order(feed_history::dsl::created_at.desc())
        .select((feed_history::dsl::etag, feed_history::dsl::last_modified))
        .first::<(Option<String>, Option<String>)>(conn)
    {
        Ok((etag, last_modified)) => Some(ConditionalGetData {
            etag,
            last_modified,
        }),
        _ => None,
    }
}

fn find_last_fetch_time(conn: &SqliteConnection, feed_url: &str) -> Option<String> {
    use crate::schema::feed_history;
    let feed_id = feed_id_from_url(feed_url);
    match feed_history::table
        .filter(feed_history::dsl::feed_id.eq(feed_id))
        .order(feed_history::dsl::created_at.desc())
        .select(feed_history::dsl::created_at)
        .first::<Option<String>>(conn)
    {
        Ok(last_fetch_time) => last_fetch_time,
        _ => None,
    }
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

    {
        use crate::models;
        use crate::schema::entries::dsl::{entries, feed_id, id};

        let entry_exists = entries
            .filter(id.eq(&entry_id))
            .count()
            .get_result::<i64>(conn)?
            > 0;

        if entry_exists {
            log::trace!("Entry exists {}", entry_id);
            diesel::update(entries)
                .filter(id.eq(&feed_id))
                .set(models::EntryUpdate {
                    defunct: Some(false),
                    published: Some(&entry_published),
                    updated_at: Some(&now.to_rfc3339()),
                    title: Some(&entry_title),
                    link: Some(&entry_link),
                    summary: Some(&entry_summary),
                    content: Some(&entry_content),
                })
                .execute(conn)?;
        } else {
            log::trace!("Entry new {}", entry_id);
            diesel::insert_into(entries)
                .values(models::EntryNew {
                    feed_id: parent_feed_id,
                    id: &entry_id,
                    defunct: false,
                    published: &entry_published,
                    created_at: &now.to_rfc3339(),
                    updated_at: &now.to_rfc3339(),
                    title: &entry_title,
                    link: &entry_link,
                    summary: &entry_summary,
                    content: &entry_content,
                })
                .execute(conn)?;
        }
    }
    Ok(())
}
