use clap::ArgMatches;
use config::Config;

// TODO: this function seems awkward
pub fn setup(app_m: &ArgMatches) -> Result<config::Config, Box<dyn std::error::Error>> {
    // TODO: just chaining straight from Config::default() raises complaints of temporary references, why?
    let mut config_default = Config::default();
    let config = config_default
        .set_default("log_level", "info")?
        .merge(config::File::with_name("config").required(false))?
        .merge(config::Environment::with_prefix("APP"))?;
    if app_m.is_present("debug") {
        config.set_default("debug", true)?;
        config.set("log_level", "debug")?;
    }
    // TODO: does this really need to be cloned? anything else complains about lifetimes
    Ok(config.clone())
}
