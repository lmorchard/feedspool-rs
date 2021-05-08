extern crate dotenv;

use std::str;
use std::time::Duration;

use clap::{App, ArgMatches};
use futures::stream::{self, StreamExt};

use dotenv::dotenv;
use std::env;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use feedspool::feeds::*;

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
        let conn = establish_connection();
        if let Err(err) = poll_one_feed(&conn, url, request_timeout).await {
            log::error!("Error polling feed {} - {}", url, err);
        } else {
            log::info!("Updated feed {}", &url);
        }
    });
    fut.await;
    log::info!("ALL DONE!");
    Ok(())
}

fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
