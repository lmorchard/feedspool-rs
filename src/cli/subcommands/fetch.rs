extern crate dotenv;

use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
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
    let concurrency_limit = 32;
    let min_fetch_period = Duration::from_secs(60 * 30);
    let request_timeout = Duration::from_secs(5);

    /*
    let mut feeds = vec![
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
    */

    if let Ok(feeds) = read_lines("./feed-urls.txt") {
        let fut =
            stream::iter(feeds).for_each_concurrent(concurrency_limit, |url_try| async move {
                if let Ok(url) = url_try {
                    log::info!("Fetching feed {}", &url);
                    let conn_try = db::connect(&config);
                    if let Err(err) = conn_try {
                        log::error!("Error connection to DB - {}", err);
                    } else if let Ok(conn) = conn_try {
                        match feeds::poll_one_feed(&conn, &url, request_timeout, min_fetch_period)
                            .await
                        {
                            Err(err) => log::error!("Error polling feed {} - {}", url, err),
                            Ok(fetched_feed) => {
                                if fetched_feed.feed.is_some() {
                                    log::info!("Updated feed {}", &url);
                                } else {
                                    log::info!("Skipped feed {}", &url);
                                }
                            }
                        }
                    }
                }
            });
        fut.await;
        log::info!("ALL DONE!");
    }
    Ok(())
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
