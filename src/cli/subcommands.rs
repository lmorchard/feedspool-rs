use clap::App;
use clap::ArgMatches;
use std::error::Error;

pub mod check;
pub mod fetch;
pub mod render;
pub mod serve;
pub mod toplinks;

pub fn setup(app: App<'static>) -> App<'static> {
    app.subcommand(fetch::app())
        .subcommand(serve::app())
        .subcommand(render::app())
        .subcommand(check::app())
        .subcommand(toplinks::app())
}

pub async fn execute(config: &config::Config, app_m: ArgMatches) -> Result<(), Box<dyn Error>> {
    // TODO: selectively skip setting up DB for certain commands?
    feedspool::db::setup(&config)?;
    match app_m.subcommand() {
        Some((fetch::NAME, sub_m)) => fetch::execute(&sub_m, &config).await,
        Some((serve::NAME, sub_m)) => serve::execute(&sub_m, &config).await,
        Some((render::NAME, sub_m)) => render::execute(&sub_m, &config).await,
        Some((check::NAME, sub_m)) => check::execute(&sub_m, &config).await,
        Some((toplinks::NAME, sub_m)) => toplinks::execute(&sub_m, &config).await,
        _ => Ok(()),
    }
}
