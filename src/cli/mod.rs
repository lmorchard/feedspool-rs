use clap::{crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg};
use std::error::Error;

pub mod config;
pub mod logging;
pub mod subcommands;

pub async fn execute() -> Result<(), Box<dyn Error>> {
    let app = subcommands::setup(app());
    let app_m = app.get_matches();
    let config = config::setup(&app_m).unwrap();
    logging::setup(&config).unwrap();
    subcommands::execute(&config, app_m).await?;
    Ok(())
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
                .long("debug")
                .about("Turn debugging information on"),
        )
}
