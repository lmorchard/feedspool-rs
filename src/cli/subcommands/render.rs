use diesel::prelude::*;
use serde::Serialize;
use std::error::Error;
use tinytemplate::TinyTemplate;

use clap::{App, ArgMatches};

use feedspool::db;
use feedspool::models;

pub const NAME: &str = "render";

pub fn app() -> App<'static> {
    App::new(NAME).about("Render feeds data as HTML")
}

static TEMPLATE: &str = r#"Entries:
{{ for item in entries -}}
  {item.entry.published} - {item.feed.title} - {item.entry.title} - {item.entry.link}
{{ endfor }}
"#;

#[derive(PartialEq, Debug, Serialize)]
struct FeedEntry {
    entry: models::Entry,
    feed: Option<models::Feed>,
}

#[derive(PartialEq, Debug, Serialize)]
struct Context {
    entries: Vec<FeedEntry>,
}

pub async fn execute(_matches: &ArgMatches, config: &config::Config) -> Result<(), Box<dyn Error>> {
    use feedspool::schema::{entries, feeds};

    let conn = db::connect(&config)?;

    let entries_result: Vec<FeedEntry> = entries::table
        .left_join(feeds::table.on(entries::feed_id.eq(feeds::id)))
        .order((entries::dsl::published.desc(), entries::dsl::updated.desc()))
        .limit(250)
        .load::<(models::Entry, Option<models::Feed>)>(&conn)?
        .into_iter()
        .map(|row| FeedEntry {
            entry: row.0,
            feed: row.1,
        })
        .collect();

    let context = Context {
        entries: entries_result,
    };

    let mut tt = TinyTemplate::new();
    tt.add_template("hello", TEMPLATE)?;

    let rendered = tt.render("hello", &context)?;
    println!("{}", rendered);

    Ok(())
}
