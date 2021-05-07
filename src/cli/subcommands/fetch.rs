extern crate dotenv;

use std::str;
use std::time::Duration;

use clap::{App, ArgMatches};
use feed_rs::model::{Entry, Feed};
use feed_rs::parser;
use futures::stream::{self, StreamExt};
use sha2::{Digest, Sha256};

use chrono::prelude::*;

use dotenv::dotenv;
use std::env;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

pub const NAME: &str = "fetch";

pub fn app() -> App<'static> {
    App::new(NAME).about("Fetch a feed")
}

pub async fn execute(
    _matches: &ArgMatches,
    _config: &config::Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let concurrency_limit = 4;
    let request_timeout = Duration::from_secs(5);

    let feeds = vec![
        "http://feeds.feedburner.com/boingboing/iBag",
        "http://feeds.laughingsquid.com/laughingsquid",
        "http://www.memeorandum.com/index.xml",
        "http://www.slate.com/rss/",
        "http://www.theverge.com/rss/index.xml",
        "http://www.wired.com/news/feeds/rss2/0,2610,,00.xml",
        "yomama",
        "http://farts.yolo/",
        "https://blog.lmorchard.com/index.rss",
    ];

    println!("Hello, world!");

    let fut = stream::iter(feeds).for_each_concurrent(concurrency_limit, |url| async move {
        log::info!("Fetching feed {}", &url);
        match fetch_feed(url, request_timeout).await {
            Ok(fetched_feed) => {
                let conn = establish_connection();
                if let Err(err) = update_feed(&conn, &fetched_feed) {
                    log::error!("Error updating feed {:?}", err);
                } else if let Err(err) = record_feed_history(&conn, &fetched_feed) {
                    log::error!("Error recording feed history {:?}", err);
                }
                log::info!("Updated feed {}", &url);
            }
            Err(err) => log::error!("FEED ERR {:?}", err),
        }
    });
    fut.await;
    log::info!("ALL DONE!");
    Ok(())
}

#[derive(Debug)]
struct FetchedFeed {
    url: String,
    status: String,
    body: String,
    feed: Feed,
    id: String,
}

async fn fetch_feed(
    url: &str,
    timeout_duration: Duration,
) -> Result<FetchedFeed, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client.get(url).timeout(timeout_duration).send().await?;
    let status = response.status();
    let body = response.text().await?;
    let feed = parser::parse(body.as_bytes())?;
    Ok(FetchedFeed {
        url: String::from(url),
        status: String::from(status.as_str()),
        id: String::from(&feed.id),
        feed,
        body,
    })
}

fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

fn update_feed(
    conn: &SqliteConnection,
    fetched_feed: &FetchedFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    use feedspool::models;
    use feedspool::schema::feeds::dsl::*;

    let now = Utc::now();

    let feed = &fetched_feed.feed;
    let feed_url = fetched_feed.url.as_str();

    let mut feed_published = String::from(&now.to_rfc3339());
    if let Some(published_date) = feed.published {
        if published_date < now {
            feed_published = published_date.to_rfc3339();
        }
    };

    let feed_title = match &feed.title {
        Some(feed_title) => &feed_title.content,
        _ => "",
    };

    let feed_link = if feed.links.len() > 0 {
        &feed.links[0].href
    } else {
        ""
    };

    let feed_id = &fetched_feed.id;

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

fn record_feed_history(
    conn: &SqliteConnection,
    fetched_feed: &FetchedFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    use feedspool::models;
    use feedspool::schema::feed_history::dsl::*;

    let now = Utc::now().to_rfc3339();

    let history_id = format!(
        "{:x}",
        Sha256::new().chain(&fetched_feed.id).chain(&now).finalize()
    );

    diesel::insert_into(feed_history)
        .values(models::FeedHistoryNew {
            id: &history_id,
            feed_id: &fetched_feed.id,
            src: &fetched_feed.body,
            status: &fetched_feed.status,
            created_at: &now,
        })
        .execute(conn)?;

    Ok(())
}

fn update_entry(
    conn: &SqliteConnection,
    parent_feed_id: &str,
    entry: &Entry,
) -> Result<(), Box<dyn std::error::Error>> {
    use feedspool::models;
    use feedspool::schema::entries::dsl::*;

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

    let entry_link = if entry.links.len() > 0 {
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
