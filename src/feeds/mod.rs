use chrono::prelude::*;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use feed_rs::model::{Entry, Feed};
use feed_rs::parser;
use sha2::{Digest, Sha256};
use std::time::Duration;

pub async fn poll_one_feed(conn: &SqliteConnection, url: &str, request_timeout: Duration) {
    let last_get_conditions = find_last_get_conditions(&conn, &url);
    match fetch_feed(url, request_timeout, last_get_conditions).await {
        Ok(fetched_feed) => {
            if fetched_feed.feed.is_none() {
                log::info!("Skipping update for {}", url);
            } else if let Err(err) = update_feed(&conn, &fetched_feed) {
                log::error!("Error updating feed {:?}", err);
            } else if let Err(err) = record_feed_history(&conn, &fetched_feed) {
                log::error!("Error recording feed history {:?}", err);
            } else {
                log::info!("Updated feed {}", &url);
            }
        }
        Err(err) => log::error!("FEED ERR {:?}", err),
    }
}

#[derive(Debug)]
pub struct FetchedFeed {
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
) -> Result<FetchedFeed, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut request = client.get(url).timeout(timeout_duration);

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
        url: String::from(url),
        status: String::from(status.as_str()),
        headers,
        feed,
        body,
    })
}

pub fn update_feed(
    conn: &SqliteConnection,
    fetched_feed: &FetchedFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::models;
    use crate::schema::feeds::dsl::*;

    let now = Utc::now();

    let feed = &fetched_feed.feed.as_ref().unwrap();
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

    let feed_id = &feed.id;

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

pub fn record_feed_history(
    conn: &SqliteConnection,
    fetched_feed: &FetchedFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::models;
    use crate::schema::feed_history::dsl::*;

    let now = Utc::now().to_rfc3339();
    let feed = &fetched_feed.feed.as_ref().unwrap();
    let headers = &fetched_feed.headers;

    let history_id = format!(
        "{:x}",
        Sha256::new().chain(&feed.id).chain(&now).finalize()
    );

    diesel::insert_into(feed_history)
        .values(models::FeedHistoryNew {
            id: &history_id,
            feed_id: &feed.id,
            src: &fetched_feed.body,
            status: &fetched_feed.status,
            etag: header_or_blank(&headers, reqwest::header::ETAG),
            last_modified: header_or_blank(&headers, reqwest::header::LAST_MODIFIED),
            created_at: &now,
        })
        .execute(conn)?;

    Ok(())
}

#[derive(Debug)]
struct ConditionalGetData {
    etag: Option<String>,
    last_modified: Option<String>,
}

fn find_last_get_conditions(conn: &SqliteConnection, feed_url: &str) -> Option<ConditionalGetData> {
    let mut feed_id_for_url = None;
    {
        use crate::schema::feeds::dsl::*;
        if let Ok(current_feed_id) = feeds
            .select(id)
            .filter(url.eq(&feed_url))
            .first::<Option<String>>(conn)
        {
            feed_id_for_url = Some(current_feed_id);
        }
    }

    if let Some(feed_id_for_url) = feed_id_for_url {
        use crate::schema::feed_history::dsl::*;
        if let Ok((feed_etag, feed_last_modified)) = feed_history
            .select((etag, last_modified))
            .filter(feed_id.eq(feed_id_for_url))
            .order(created_at.desc())
            .first::<(Option<String>, Option<String>)>(conn)
        {
            return Some(ConditionalGetData {
                etag: feed_etag,
                last_modified: feed_last_modified,
            });
        }
    }

    None
}

fn update_entry(
    conn: &SqliteConnection,
    parent_feed_id: &str,
    entry: &Entry,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::models;
    use crate::schema::entries::dsl::*;

    let now = Utc::now();

    let mut entry_published = String::from(&now.to_rfc3339());
    if let Some(published_date) = entry.published {
        if published_date < now {
            entry_published = published_date.to_rfc3339();
        }
    };

    let entry_title = match &entry.title {
        Some(t) => &t.content,
        _ => "",
    };

    let entry_link = if !entry.links.is_empty() {
        &entry.links[0].href
    } else {
        ""
    };

    let entry_summary = match &entry.summary {
        Some(x) => &x.content,
        _ => "",
    };

    // TODO: ughhhhh this is terrible
    let entry_content = if entry.content.is_some() && entry.content.as_ref().unwrap().body.is_some()
    {
        &entry.content.as_ref().unwrap().body.as_ref().unwrap()[..]
    } else {
        ""
    };

    let entry_id = format!(
        "{:x}",
        Sha256::new()
            .chain(&entry.id)
            .chain(&entry_title)
            .chain(&entry_link)
            .finalize()
    );

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

    Ok(())
}
