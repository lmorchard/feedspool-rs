use clap::ArgMatches;
use clap::{App};

pub mod fetch;

pub fn setup(app: App<'static>) -> App<'static> {
  app.subcommand(fetch::app())
}

pub async fn execute(
  config: &config::Config,
  app_m: ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
  match app_m.subcommand() {
    Some((fetch::NAME, sub_m)) => fetch::execute(&sub_m, &config).await,
    _ => Ok(()),
  }
}
