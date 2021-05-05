pub fn setup(config: &config::Config) -> Result<(), Box<dyn std::error::Error>> {
    let default_level = config.get::<String>("log_level")?;
    let env_with_defaults = env_logger::Env::default().default_filter_or(default_level);
    env_logger::Builder::from_env(env_with_defaults).init();
    Ok(())
}
