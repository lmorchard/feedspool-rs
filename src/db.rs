use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::embed_migrations;

embed_migrations!("migrations");

/// # Errors
///
/// Will return Err for any problem in connection to database
pub fn setup(config: &config::Config) -> Result<SqliteConnection, Box<dyn std::error::Error>> {
    let debug = config.get_bool("debug")?;
    let conn = connect(config)?;
    if debug {
        embedded_migrations::run_with_output(&conn, &mut std::io::stdout())?;
    } else {
        embedded_migrations::run(&conn)?;
    }
    Ok(conn)
}

/// # Errors
///
/// Will return Err for any problem in connection to database
pub fn connect(config: &config::Config) -> Result<SqliteConnection, Box<dyn std::error::Error>> {
    let database_url = &config.get_str("database_url")?;
    Ok(SqliteConnection::establish(&database_url)?)
}
