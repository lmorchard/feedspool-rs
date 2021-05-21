use clap::{App, Arg, ArgMatches};
use std::error::Error;
use std::str;
use std::time::Duration;

use feedspool::{db, feeds};

pub const NAME: &str = "check";

pub fn app() -> App<'static> {
    App::new(NAME).about("Check parsing for a feed").arg(
        Arg::new("url")
            .long("url")
            .about("Feed URL")
            .takes_value(true),
    )
}

pub async fn execute(matches: &ArgMatches, config: &config::Config) -> Result<(), Box<dyn Error>> {
    let request_timeout = Duration::from_secs(5);

    if let Some(url) = matches.value_of("url") {
        log::info!("Fetching {}", &url);
        let conn = db::connect(&config).unwrap();

        let last_get_conditions = db::find_last_get_conditions(&conn, &url);
        let fetch_result = feeds::fetch_feed(url, request_timeout, last_get_conditions).await;
        if let Ok(result) = fetch_result {
            if let feeds::result::FeedPollResult::Fetched { feed, .. } = result {
                log::info!("Feed: {:?}", serde_json::to_string(&feed).unwrap())
            } else {
                log::info!("Fetch result: {:?}", &result);
            }
        } else {
            log::info!("Fetch result: {:?}", &fetch_result);
        }
    } else {
        log::error!("--url is required");
    }
    Ok(())
}
