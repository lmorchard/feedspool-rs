extern crate clap;
extern crate config;

mod cli;

use std::error::Error;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    cli::execute().await
}
