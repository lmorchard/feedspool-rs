use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::str;
use std::time::Duration;

use clap::{App, Arg, ArgMatches};
use futures::stream::{self, StreamExt};

use feedspool::feeds::result::{FeedPollError, FeedPollResult};
use feedspool::{db, feeds};

pub const NAME: &str = "fetch";

pub fn app() -> App<'static> {
    App::new(NAME).about("Fetch a feed").arg(
        Arg::new("feeds")
            .long("feeds")
            .about("Filename of feeds list")
            .takes_value(true),
    )
}

pub async fn execute(matches: &ArgMatches, config: &config::Config) -> Result<(), Box<dyn Error>> {
    let concurrency_limit = config.get::<usize>("fetch_concurrency_limit")?;
    let min_fetch_period = Duration::from_secs(config.get("fetch_min_fetch_period")?);
    let request_timeout = Duration::from_secs(config.get("fetch_request_timeout")?);
    let fetch_retain_src = config.get::<bool>("fetch_retain_src")?;

    let feeds_filename = match matches.value_of("feeds") {
        Some(filename) => String::from(filename),
        None => config.get("fetch_feeds_filename")?,
    };

    let feeds = read_lines(feeds_filename)?;
    let fut = stream::iter(feeds).for_each_concurrent(concurrency_limit, |url_try| async move {
        if let Ok(url) = url_try {
            log::info!("Fetching {}", &url);
            let conn_try = db::connect(&config);
            if let Err(err) = conn_try {
                log::error!("Error connection to DB - {}", err);
            } else if let Ok(conn) = conn_try {
                match feeds::poll_one_feed(
                    &conn,
                    &url,
                    request_timeout,
                    min_fetch_period,
                    fetch_retain_src,
                )
                .await
                {
                    Ok(fetch_result) => match fetch_result {
                        FeedPollResult::Skipped => {
                            log::info!("Skipped update for {}", url)
                        }
                        FeedPollResult::NotModified { .. } => {
                            log::info!("No updates for {}", url)
                        }
                        FeedPollResult::Updated { .. } => log::info!("Updated {}", url),
                        _ => log::info!("Unexpected result {} {:?}", url, fetch_result),
                    },
                    Err(error) => match error {
                        FeedPollError::FetchFailed { fetch } => {
                            log::error!("Fetch failed with status {} for {}", fetch.status, url)
                        }
                        FeedPollError::NotFound(_) => {
                            log::error!("Not found error for {}", url)
                        }
                        FeedPollError::Timedout(_) => {
                            log::error!("Fetch timed out for {}", url)
                        }
                        FeedPollError::ParseError { error, .. } => {
                            log::error!("Feed parsing failed for {} - {:?}", url, error)
                        }
                        FeedPollError::UpdateError { error, .. } => {
                            log::error!("Databse update failed for {} - {:?}", url, error)
                        }
                        _ => log::error!("Error polling feed {} - {:?}", url, error),
                    },
                }
            }
        }
    });
    fut.await;
    log::info!("ALL DONE!");
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
