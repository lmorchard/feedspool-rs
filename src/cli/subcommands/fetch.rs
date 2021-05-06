extern crate dotenv;

use std::str;
use std::time::Duration;

use clap::{App, ArgMatches};
use feed_rs::model::Feed;
use feed_rs::parser;
use futures::stream::{self, StreamExt};

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
    ];

    println!("Hello, world!");

    let fut = stream::iter(feeds).for_each_concurrent(concurrency_limit, |url| async move {
        match fetch_feed(url, request_timeout).await {
            Ok(feed) => {
                log::info!("FEED {:?} {:?}", feed.id, feed.title);
                if let Ok(_) = update_feed(&url, &feed) {
                    log::info!("FEED UPDATED {:?} {:?}", feed.id, feed.title);
                }
            }
            Err(err) => log::error!("FEED ERR {:?}", err),
        }
    });
    fut.await;
    log::info!("ALL DONE!");
    Ok(())
}

pub async fn fetch_feed(
    url: &str,
    timeout_duration: Duration,
) -> Result<Feed, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client.get(url).timeout(timeout_duration).send().await?;
    let body = response.text().await?;
    let feed = parser::parse(body.as_bytes())?;
    Ok(feed)
}

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub fn update_feed(url: &str, feed: &Feed) -> Result<(), Box<dyn std::error::Error>> {
    use feedspool::models;

    let conn = establish_connection();

    let updated_at = Utc::now().to_rfc3339(); 

    let title = if let Some(feed_title) = &feed.title {
        &feed_title.content
    } else {
        ""
    };

    let link = if feed.links.len() > 0 {
        &feed.links[0].href
    } else {
        ""
    };

    let new_feed = models::NewFeed {
        url: url,
        created_at: &updated_at,
        updated_at: &updated_at,
        id: &feed.id,
        title: &title,
        link: &link,
    };

    let insert_count = diesel::replace_into(feedspool::schema::feeds::table)
        .values(&new_feed)
        .execute(&conn)?;

    log::debug!("FEED INSERTED {}", insert_count);

    Ok(())
}
