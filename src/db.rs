use chrono::prelude::*;
use diesel::prelude::*;
use diesel_migrations::embed_migrations;
use std::collections::HashSet;
use std::error::Error;
use std::hash::BuildHasher;

use diesel::{
    r2d2::{ConnectionManager, Pool},
    sqlite::SqliteConnection,
};

use crate::feeds::result::{ConditionalGetData, FeedFetchResult, FeedPollError};

use sha2::{Digest, Sha256};

embed_migrations!("migrations");

pub mod paginate_dsl;

pub type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

/// # Errors
///
/// Will return Err for any problem in connection to database
pub fn setup(config: &config::Config) -> Result<SqliteConnection, Box<dyn Error>> {
    let debug = config.get_bool("debug")?;
    let conn = connect(config)?;
    if debug {
        embedded_migrations::run_with_output(&conn, &mut std::io::stdout())?;
    } else {
        embedded_migrations::run(&conn)?;
    }
    Ok(conn)
}

/// # Errors
///
/// Will return Err for any problem in connection to database
pub fn connect(config: &config::Config) -> Result<SqliteConnection, Box<dyn Error>> {
    let database_url = &config.get_str("database_url")?;
    Ok(SqliteConnection::establish(&database_url)?)
}

/// # Errors
///
/// Will return Err for problems getting `database_url` from config or creating database pool
pub fn create_pool(config: &config::Config) -> Result<SqlitePool, Box<dyn Error>> {
    let database_url = &config.get_str("database_url")?;
    Ok(SqlitePool::builder()
        .max_size(8)
        .build(ConnectionManager::new(database_url))?)
}

#[must_use]
pub fn feed_id_from_url(url: &str) -> String {
    format!("{:x}", Sha256::new().chain(url).finalize())
}

/// # Errors
///
/// Returns `diesel::result::Error` for any DB failure
pub fn upsert_feed(
    conn: &SqliteConnection,
    upsert: &crate::models::FeedUpsert,
) -> Result<(), diesel::result::Error> {
    use crate::models;
    use crate::schema::feeds::dsl::{feeds, id};

    let feed_exists = feeds
        .filter(id.eq(&upsert.id))
        .count()
        .get_result::<i64>(conn)?
        > 0;

    if feed_exists {
        log::trace!("Feed exists {}", &upsert.id);
        diesel::update(feeds)
            .filter(id.eq(&upsert.id))
            .set(models::FeedUpdate {
                json: Some(&upsert.json),
                title: Some(&upsert.title),
                link: Some(&upsert.link),
                url: Some(&upsert.url),
                published: Some(&upsert.published),
                updated: Some(&upsert.updated),
                modified_at: Some(&upsert.now),
                last_entry_published: Some(&upsert.last_entry_published),
            })
            .execute(conn)?;
    } else {
        log::trace!("Feed new {}", &upsert.id);
        diesel::insert_into(feeds)
            .values(models::FeedNew {
                id: &upsert.id,
                json: &upsert.json,
                title: &upsert.title,
                link: &upsert.link,
                url: &upsert.url,
                published: &upsert.published,
                created_at: &upsert.now,
                modified_at: &upsert.now,
                last_entry_published: &upsert.last_entry_published,
            })
            .execute(conn)?;
    }
    Ok(())
}

/// # Errors
///
/// Returns `diesel::result::Error` for any DB failure
pub fn upsert_entry(
    conn: &SqliteConnection,
    upsert: &crate::models::EntryUpsert,
) -> Result<(), diesel::result::Error> {
    use crate::models;
    use crate::schema::entries::dsl::{entries, id};

    let entry_exists = entries
        .filter(id.eq(&upsert.id))
        .count()
        .get_result::<i64>(conn)?
        > 0;

    if entry_exists {
        log::trace!("Entry exists {}", &upsert.id);
        if !&upsert.skip_update {
            diesel::update(entries)
                .filter(id.eq(&upsert.id))
                .set(models::EntryUpdate {
                    defunct: Some(false),
                    json: Some(&upsert.json),
                    title: Some(&upsert.title),
                    link: Some(&upsert.link),
                    summary: Some(&upsert.summary),
                    content: Some(&upsert.content),
                    published: Some(&upsert.published),
                    updated: Some(&upsert.updated),
                    modified_at: Some(&upsert.now),
                })
                .execute(conn)?;
        }
    } else {
        log::trace!("Entry new {}", &upsert.id);
        diesel::insert_into(entries)
            .values(models::EntryNew {
                id: &upsert.id,
                feed_id: &upsert.feed_id,
                defunct: false,
                json: &upsert.json,
                title: &upsert.title,
                link: &upsert.link,
                summary: &upsert.summary,
                content: &upsert.content,
                published: &upsert.published,
                updated: &upsert.updated,
                modified_at: &upsert.now,
                created_at: &upsert.now,
            })
            .execute(conn)?;
    }
    Ok(())
}

/// # Errors
///
/// Returns `diesel::result::Error` for any DB failure
pub fn mark_old_entries_defunct<S: BuildHasher>(
    conn: &SqliteConnection,
    parent_feed_id: &str,
    seen_entry_ids: HashSet<String, S>,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::entries::dsl::{defunct, entries, feed_id, id};
    diesel::update(entries)
        .filter(feed_id.eq(parent_feed_id).and(id.ne_all(seen_entry_ids)))
        .set(defunct.eq(true))
        .execute(conn)
}

/// # Errors
///
/// Returns `FeedPollError::DatabaseError` for any DB failure
pub fn insert_feed_history(
    conn: &SqliteConnection,
    fetch: &FeedFetchResult,
    retain_src: bool,
) -> Result<(), FeedPollError> {
    let now = Utc::now().to_rfc3339();
    let history_id = &format!(
        "{:x}",
        Sha256::new().chain(&fetch.id).chain(&now).finalize()
    );
    {
        use crate::models;
        use crate::schema::feed_history;
        let mut src = "";
        if retain_src {
            src = &fetch.body;
        }
        if let Err(db_error) = diesel::insert_into(feed_history::table)
            .values(models::FeedHistoryNewSuccess {
                id: history_id,
                feed_id: &fetch.id,
                src: &src,
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

/// # Errors
///
/// Returns `FeedPollError::DatabaseError` for any DB failure
pub fn insert_feed_history_error(
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
        Err(_) => None,
        Ok((etag, last_modified)) => Some(ConditionalGetData {
            etag,
            last_modified,
        }),
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
        Err(_) => None,
        Ok(last_fetch_time) => last_fetch_time,
    }
}
