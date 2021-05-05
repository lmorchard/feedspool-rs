use std::str;

use clap::{App, ArgMatches};

pub const NAME: &str = "fetch";

pub fn app() -> App<'static> {
    App::new(NAME).about("Fetch a feed")
}

pub fn execute(
    _matches: &ArgMatches,
    _config: &config::Config,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    Ok(())
}
