use std::str;

use clap::{App, ArgMatches};

pub const NAME: &str = "fetch";

pub fn app() -> App<'static> {
    App::new(NAME).about("Fetch a feed")
}

pub async fn execute(
    _matches: &ArgMatches,
    _config: &config::Config,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    let body = reqwest::get("https://www.rust-lang.org")
        .await?
        .text()
        .await?;
    println!("body = {:?}", body);
    Ok(())
}
