use clap::App;
use clap::ArgMatches;
use std::error::Error;

pub mod fetch;

pub fn setup(app: App<'static>) -> App<'static> {
    app.subcommand(fetch::app())
}

pub async fn execute(config: &config::Config, app_m: ArgMatches) -> Result<(), Box<dyn Error>> {
    // TODO: selectively skip setting up DB for certain commands?
    feedspool::db::setup(&config)?;
    match app_m.subcommand() {
        Some((fetch::NAME, sub_m)) => fetch::execute(&sub_m, &config).await,
        _ => Ok(()),
    }
}
