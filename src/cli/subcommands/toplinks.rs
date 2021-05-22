use chrono::prelude::*;
use diesel::prelude::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::error::Error;
// use tinytemplate::TinyTemplate;

use clap::{App, ArgMatches};

use feedspool::db;
use feedspool::models;

pub const NAME: &str = "toplinks";

pub fn app() -> App<'static> {
    App::new(NAME).about("Report top links in feeds")
}

/*
static TEMPLATE: &str = r#"Entries:
{{ for item in entries -}}
{item.entry.published} - {item.feed.title} - {item.entry.title} - {item.entry.link}
  {{ for link in item.links -}}
  * {link}
  {{ endfor }}
{{ endfor }}
"#;
*/

#[derive(PartialEq, Debug, Serialize)]
struct FeedEntry {
    feed: Option<models::Feed>,
    entry: models::Entry,
    links: HashSet<String>,
}

#[derive(PartialEq, Debug, Serialize)]
struct Context {
    entries: Vec<FeedEntry>,
}

pub async fn execute(_matches: &ArgMatches, config: &config::Config) -> Result<(), Box<dyn Error>> {
    use feedspool::schema::{entries, feeds};

    let conn = db::connect(&config)?;

    let now = Utc::now();
    let since_datetime = now - chrono::Duration::days(30);
    let links_count_threshold = 3;

    let entries_by_id = entries::table
        .filter(entries::published.gt(&since_datetime.to_rfc3339()))
        .left_join(feeds::table.on(entries::feed_id.eq(feeds::id)))
        .order((entries::dsl::published.desc(), entries::dsl::updated.desc()))
        .load::<(models::Entry, Option<models::Feed>)>(&conn)?
        .into_iter()
        .map(&handle_entry)
        .collect::<Vec<FeedEntry>>();

    /*
    let ids = entries_by_id
        .iter()
        .fold(HashMap::new(), |mut map, feed_entry| {
            if let Some(entry_id) = &feed_entry.entry.id {
                for link in &feed_entry.links {
                    map.entry(String::from(link))
                        .or_insert_with(HashSet::new)
                        .insert(String::from(entry_id));
                }
            }
            map
        });
    */

    let ids = entries_by_id
        .iter()
        .fold(HashMap::new(), |mut map, feed_entry| {
            if let Some(feed) = &feed_entry.feed {
                if let Some(feed_id) = &feed.title {
                    for link in &feed_entry.links {
                        map.entry(String::from(link))
                            .or_insert_with(HashSet::new)
                            .insert(String::from(feed_id));
                    }
                }
            }
            map
        });

    let mut top_links = ids.iter().fold(Vec::new(), |mut list, (link, ids)| {
        if ids.len() >= links_count_threshold {
            list.push((link, ids));
        }
        list
    });

    top_links.sort_by(|a, b| {
        let a_len = a.1.len();
        let b_len = b.1.len();
        b_len.partial_cmp(&a_len).unwrap()
    });

    for (link, ids) in top_links {
        println!("* ({}) {}", ids.len(), link);
        println!("    * {:?}", ids);
    }
    /*
    let context = Context {
      entries: entries_result,
    };

    let mut tt = TinyTemplate::new();
    tt.add_template("hello", TEMPLATE)?;

    let rendered = tt.render("hello", &context)?;
    println!("{}", rendered);
    */

    Ok(())
}

fn handle_entry(row: (models::Entry, Option<models::Feed>)) -> FeedEntry {
    use scraper::{Html, Selector};
    use url::Url;

    let entry = row.0;
    let feed = row.1;

    let entry_url = Url::parse(&entry.link.as_ref().unwrap_or(&String::from("")));

    let feed_url = Url::parse(match &feed {
        Some(feed) => match &feed.link {
            Some(link) => link,
            None => &"",
        },
        None => &"",
    });

    let mut links: HashSet<String> = HashSet::new();
    if let Some(entry_link) = &entry.link {
        links.insert(String::from(entry_link));
    }

    for content in &[&entry.content, &entry.summary] {
        if let Some(content) = content {
            let fragment = Html::parse_fragment(&content);
            let selector = Selector::parse("a").unwrap();
            for element_ref in fragment.select(&selector) {
                let element = element_ref.value();
                if let Some(link) = element.attr("href") {
                    let link_url = Url::parse(&link);
                    if let Ok(mut link_url) = link_url {
                        if let Ok(entry_url) = &entry_url {
                            if link_url.origin() == entry_url.origin() {
                                continue;
                            }
                        }
                        if let Ok(feed_url) = &feed_url {
                            if link_url.origin() == feed_url.origin() {
                                continue;
                            }
                        }
                        link_url.set_fragment(None);
                        //link_url.set_query(None);
                        links.insert(String::from(link_url));
                    }
                }
            }
        }
    }

    FeedEntry { entry, feed, links }
}
