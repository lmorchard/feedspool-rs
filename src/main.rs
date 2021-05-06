use exitfailure::ExitFailure;

extern crate clap;
extern crate config;

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
