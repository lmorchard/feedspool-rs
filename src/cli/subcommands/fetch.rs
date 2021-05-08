extern crate dotenv;

use std::error::Error;
use std::str;
use std::time::Duration;

use clap::{App, ArgMatches};
use futures::stream::{self, StreamExt};

use feedspool::{db, feeds};

pub const NAME: &str = "fetch";

pub fn app() -> App<'static> {
    App::new(NAME).about("Fetch a feed")
}

pub async fn execute(_matches: &ArgMatches, config: &config::Config) -> Result<(), Box<dyn Error>> {
    let concurrency_limit = 4;
    let request_timeout = Duration::from_secs(5);

    let feeds = vec![
        "http://feeds.feedburner.com/boingboing/iBag",
        "http://feeds.laughingsquid.com/laughingsquid",
        "http://www.memeorandum.com/index.xml",
        "http://www.slate.com/rss/",
        "http://www.theverge.com/rss/index.xml",
        "http://www.wired.com/news/feeds/rss2/0,2610,,00.xml",
        //"yomama",
        //"http://farts.yolo/",
        "https://blog.lmorchard.com/index.rss",
    ];

    let fut = stream::iter(feeds).for_each_concurrent(concurrency_limit, |url| async move {
        log::info!("Fetching feed {}", &url);
        let conn_try = db::connect(&config);
        if let Err(err) = conn_try {
            log::error!("Error connection to DB - {}", err);
        } else if let Ok(conn) = conn_try {
            if let Err(err) = feeds::poll_one_feed(&conn, url, request_timeout).await {
                log::error!("Error polling feed {} - {}", url, err);
            } else {
                log::info!("Updated feed {}", &url);
            }
        }
    });
    fut.await;
    log::info!("ALL DONE!");
    Ok(())
}
