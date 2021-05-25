use super::schema::{entries, feed_history, feeds};
use juniper::GraphQLObject;
use serde::{Deserialize, Serialize};

// TODO: rework schema to make most fields non-nullable?

#[derive(Queryable, PartialEq, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: Option<String>,
    pub feed_id: Option<String>,
    pub published: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub defunct: Option<bool>,
    pub json: Option<String>,
    pub guid: Option<String>,
    pub title: Option<String>,
    pub link: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub updated: Option<String>,
}

pub struct EntryUpsert<'a> {
    pub skip_update: bool,
    pub id: &'a str,
    pub feed_id: &'a str,
    pub json: &'a str,
    pub title: &'a str,
    pub link: &'a str,
    pub summary: &'a str,
    pub content: &'a str,
    pub published: &'a str,
    pub updated: &'a str,
    pub now: &'a str,
}

#[derive(Insertable)]
#[table_name = "entries"]
pub struct EntryNew<'a> {
    pub id: &'a str,
    pub feed_id: &'a str,
    pub published: &'a str,
    pub updated: &'a str,
    pub created_at: &'a str,
    pub modified_at: &'a str,
    pub defunct: bool,
    pub title: &'a str,
    pub link: &'a str,
    pub summary: &'a str,
    pub content: &'a str,
    pub json: &'a str,
}

#[derive(AsChangeset)]
#[table_name = "entries"]
pub struct EntryUpdate<'a> {
    pub published: Option<&'a str>,
    pub updated: Option<&'a str>,
    pub modified_at: Option<&'a str>,
    pub defunct: Option<bool>,
    pub title: Option<&'a str>,
    pub link: Option<&'a str>,
    pub summary: Option<&'a str>,
    pub content: Option<&'a str>,
    pub json: Option<&'a str>,
}

#[derive(Queryable, PartialEq, Debug, Serialize, Deserialize, GraphQLObject)]
#[graphql(description = "An event in feed fetch history")]
pub struct FeedHistory {
    pub id: Option<String>,
    pub feed_id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub src: Option<String>,
    pub status: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub json: Option<String>,
    pub is_error: Option<bool>,
    pub error_text: Option<String>,
}

#[derive(Insertable)]
#[table_name = "feed_history"]
pub struct FeedHistoryNewSuccess<'a> {
    pub id: &'a str,
    pub feed_id: &'a str,
    pub created_at: &'a str,
    pub src: &'a str,
    pub status: &'a str,
    pub etag: &'a str,
    pub last_modified: &'a str,
}

#[derive(Insertable)]
#[table_name = "feed_history"]
pub struct FeedHistoryNewError<'a> {
    pub id: &'a str,
    pub feed_id: &'a str,
    pub created_at: &'a str,
    pub is_error: bool,
    pub error_text: &'a str,
}

#[derive(Queryable, PartialEq, Debug, Serialize, Deserialize)]
pub struct Feed {
    pub id: Option<String>,
    pub published: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub url: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub link: Option<String>,
    pub json: Option<String>,
    pub updated: Option<String>,
    pub last_entry_published: Option<String>,
}

pub struct FeedUpsert<'a> {
    pub id: &'a str,
    pub json: &'a str,
    pub title: &'a str,
    pub link: &'a str,
    pub url: &'a str,
    pub published: &'a str,
    pub updated: &'a str,
    pub now: &'a str,
    pub last_entry_published: &'a str,
}

#[derive(Insertable)]
#[table_name = "feeds"]
pub struct FeedNew<'a> {
    pub id: &'a str,
    pub published: &'a str,
    pub created_at: &'a str,
    pub modified_at: &'a str,
    pub url: &'a str,
    pub title: &'a str,
    pub link: &'a str,
    pub json: &'a str,
    pub last_entry_published: &'a str,
}

#[derive(AsChangeset)]
#[table_name = "feeds"]
pub struct FeedUpdate<'a> {
    pub published: Option<&'a str>,
    pub updated: Option<&'a str>,
    pub modified_at: Option<&'a str>,
    pub url: Option<&'a str>,
    pub title: Option<&'a str>,
    pub link: Option<&'a str>,
    pub json: Option<&'a str>,
    pub last_entry_published: Option<&'a str>,
}
