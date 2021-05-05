extern crate clap;
extern crate config;

mod cli;
use cli::subcommands;

fn main() {
    let app = cli::setup();
    let app_m = app.get_matches();
    let config = cli::config::setup(&app_m).unwrap();
    cli::logging::setup(&config).unwrap();
    match app_m.subcommand() {
        Some((subcommands::fetch::NAME, sub_m)) => subcommands::fetch::execute(&sub_m, &config),
        _ => Ok(()),
    }
    .unwrap()
}
