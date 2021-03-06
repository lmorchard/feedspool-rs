use std::error::Error;

pub fn setup(config: &config::Config) -> Result<(), Box<dyn Error>> {
    let default_level = config.get_str("log_level")?;
    let env_with_defaults = env_logger::Env::default().default_filter_or(default_level);
    env_logger::Builder::from_env(env_with_defaults).init();
    Ok(())
}
