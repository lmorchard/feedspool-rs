use chrono::prelude::*;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use feed_rs::model::{Entry, Feed};
use feed_rs::parser;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::time::Duration;

/// # Errors
///
/// Will return Err for any failure while polling a feed
/// TODO: actually inventory and document the errors here
pub async fn poll_one_feed(
    conn: &SqliteConnection,
    url: &str,
    request_timeout: Duration,
) -> Result<FetchedFeed, Box<dyn Error>> {
    let last_get_conditions = find_last_get_conditions(&conn, &url);
    let fetched_feed = fetch_feed(url, request_timeout, last_get_conditions).await?;
    record_feed_history(&conn, &fetched_feed)?;
    if fetched_feed.feed.is_some() {
        update_feed(&conn, &fetched_feed)?;
    }
    Ok(fetched_feed)
}

fn feed_id_from_url(url: &str) -> String {
    format!("{:x}", Sha256::new().chain(url).finalize())
}

#[derive(Debug)]
pub struct FetchedFeed {
    id: String,
    url: String,
    status: String,
    body: String,
    headers: reqwest::header::HeaderMap,
    feed: Option<Feed>,
}

async fn fetch_feed(
    url: &str,
    timeout_duration: Duration,
    last_get_conditions: Option<ConditionalGetData>,
) -> Result<FetchedFeed, Box<dyn Error>> {
    let mut request = reqwest::Client::new().get(url).timeout(timeout_duration);
    if let Some(last_get_conditions) = last_get_conditions {
        if let Some(etag) = last_get_conditions.etag {
            request = request.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        if let Some(last_modified) = last_get_conditions.last_modified {
            request = request.header(reqwest::header::IF_MODIFIED_SINCE, last_modified);
        }
    }

    let response = request.send().await?;
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await?;

    let feed = match status {
        reqwest::StatusCode::OK => Some(parser::parse(body.as_bytes())?),
        _ => None,
    };
    Ok(FetchedFeed {
        id: feed_id_from_url(&url),
        url: String::from(url),
        status: String::from(status.as_str()),
        headers,
        body,
        feed,
    })
}

fn update_feed(conn: &SqliteConnection, fetched_feed: &FetchedFeed) -> Result<(), Box<dyn Error>> {
    let now = Utc::now();

    let feed = &fetched_feed.feed.as_ref().unwrap();
    let feed_id = &fetched_feed.id;
    let feed_url = fetched_feed.url.as_str();

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

    {
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
    }

    for entry in &feed.entries {
        update_entry(&conn, &feed_id, &entry)?;
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
    fetched_feed: &FetchedFeed,
) -> Result<(), Box<dyn Error>> {
    let now = Utc::now().to_rfc3339();
    let headers = &fetched_feed.headers;
    let feed_id = &fetched_feed.id;
    let history_id = &format!("{:x}", Sha256::new().chain(&feed_id).chain(&now).finalize());
    {
        use crate::models;
        use crate::schema::feed_history;
        diesel::insert_into(feed_history::table)
            .values(models::FeedHistoryNew {
                feed_id,
                id: history_id,
                src: &fetched_feed.body,
                status: &fetched_feed.status,
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

fn update_entry(
    conn: &SqliteConnection,
    parent_feed_id: &str,
    entry: &Entry,
) -> Result<(), Box<dyn Error>> {
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
