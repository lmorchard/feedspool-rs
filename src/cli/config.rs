use clap::ArgMatches;
use config::Config;
use std::error::Error;

pub fn setup(app_m: &ArgMatches) -> Result<config::Config, Box<dyn Error>> {
    let mut config = Config::default();
    config
        .set_default("debug", false)?
        .set_default("log_level", "info")?
        .set_default("database_url", "feedspool.sqlite")?
        // TODO: split this up so subcommands can contribute defaults?
        .set_default("http_server_address", "0.0.0.0:3010")?
        .set_default("http_server_static_path", "./www/")?
        .set_default("fetch_feeds_filename", "feed-urls.txt")?
        .set_default("fetch_retain_src", false)?
        .set_default("fetch_skip_entry_update", true)?
        .set_default("fetch_min_fetch_period", 60 * 30)?
        .set_default("fetch_request_timeout", 5)?
        .set_default("fetch_concurrency_limit", 16)?
        .merge(config::File::with_name("config").required(false))?
        .merge(config::Environment::with_prefix("APP"))?;

    if app_m.is_present("debug") {
        config.set_default("debug", true)?;
        config.set("log_level", "debug")?;
    }

    Ok(config)
}
