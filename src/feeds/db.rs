// TODO: cobble together some model types for the upsert methods
#![allow(clippy::too_many_arguments)]

use chrono::prelude::*;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::feeds::result::{ConditionalGetData, FeedFetchResult, FeedPollError};

use sha2::{Digest, Sha256};

pub fn feed_id_from_url(url: &str) -> String {
    format!("{:x}", Sha256::new().chain(url).finalize())
}

pub fn upsert_feed(
    conn: &SqliteConnection,
    now: &DateTime<Utc>,
    // TODO: wrap up these args in a model type
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

pub fn upsert_entry(
    conn: &SqliteConnection,
    now: &DateTime<Utc>,
    // TODO: wrap up these args in a model type
    parent_feed_id: &str,
    entry_id: &str,
    entry_published: &str,
    entry_title: &str,
    entry_link: &str,
    entry_summary: &str,
    entry_content: &str,
) -> Result<(), diesel::result::Error> {
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
    Ok(())
}

pub fn insert_feed_history(
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

pub fn find_last_get_conditions(
    conn: &SqliteConnection,
    feed_url: &str,
) -> Option<ConditionalGetData> {
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

pub fn find_last_fetch_time(conn: &SqliteConnection, feed_url: &str) -> Option<String> {
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
