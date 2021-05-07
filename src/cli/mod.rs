use clap::ArgMatches;
use clap::{crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg};

pub mod config;
pub mod logging;
pub mod subcommands;

// TODO: no idea what I'm going with these lifetimes
pub fn setup() -> App<'static> {
    subcommands::setup(app())
}

pub async fn execute(
    config: &::config::Config,
    app_m: ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    subcommands::execute(&config, app_m).await
}

fn app() -> App<'static> {
    App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .author(crate_authors!())
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(
            Arg::new("debug")
                .short('d')
                .about("Turn debugging information on"),
        )
}
