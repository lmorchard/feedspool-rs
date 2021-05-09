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
pub struct FeedFetchResult {
    pub id: String,
    pub url: String,
    pub status: String,
    pub headers: reqwest::header::HeaderMap,
    pub body: String,
}

#[derive(Debug)]
pub enum FeedPollResult {
    Skipped,
    NotModified { fetch: FeedFetchResult },
    Fetched { fetch: FeedFetchResult, feed: Feed },
    Updated { fetch: FeedFetchResult, feed: Feed },
}

impl FeedPollResult {
    /// # Panics
    ///
    /// Will panic if attempted on something other than `FeedPollResult::Fetched`
    #[must_use]
    pub fn fetched_to_updated(self) -> FeedPollResult {
        match self {
            Self::Fetched { fetch, feed } => Self::Updated { fetch, feed },
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
            Self::Fetched { fetch, feed } => FeedPollError::UpdateError { fetch, feed, error },
            // TODO: find a non-panic way to handle this?
            _ => panic!("fetched_to_update_error {:?}", self),
        }
    }
}

#[derive(Debug)]
pub enum FeedPollError {
    FetchTimeError(time::OutOfRangeError),
    Timedout(reqwest::Error),
    NotFound(reqwest::Error),
    FetchError(reqwest::Error),
    DatabaseError(diesel::result::Error),
    FetchFailed {
        fetch: FeedFetchResult,
    },
    ParseError {
        fetch: FeedFetchResult,
        error: feed_rs::parser::ParseFeedError,
    },
    UpdateError {
        fetch: FeedFetchResult,
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

fn insert_feed_history(
    conn: &SqliteConnection,
    fetch: &FeedFetchResult,
) -> Result<(), FeedPollError> {
    let now = Utc::now().to_rfc3339();
    let history_id = &format!(
        "{:x}",
        Sha256::new().chain(&fetch.id).chain(&now).finalize()
    );
    {
        use crate::models;
        use crate::schema::feed_history;
        if let Err(db_error) = diesel::insert_into(feed_history::table)
            .values(models::FeedHistoryNewSuccess {
                id: history_id,
                feed_id: &fetch.id,
                src: &fetch.body,
                status: &fetch.status,
                etag: header_or_blank(&fetch.headers, reqwest::header::ETAG),
                last_modified: header_or_blank(&fetch.headers, reqwest::header::LAST_MODIFIED),
                created_at: &now,
            })
            .execute(conn)
        {
            return Err(FeedPollError::DatabaseError(db_error));
        }
    }
    Ok(())
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
