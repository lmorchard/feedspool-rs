#[macro_use]
extern crate diesel_migrations;
extern crate clap;
extern crate config;

use diesel_migrations::embed_migrations;
use exitfailure::ExitFailure;

embed_migrations!("migrations");

mod cli;

#[tokio::main]
async fn main() -> Result<(), ExitFailure> {
    let app = cli::setup();
    let app_m = app.get_matches();
    let config = cli::config::setup(&app_m).unwrap();
    cli::logging::setup(&config).unwrap();
    cli::execute(&config, app_m).await.unwrap();
    Ok(())
}
