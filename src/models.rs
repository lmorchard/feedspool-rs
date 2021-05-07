use super::schema::{entries, feed_history, feeds};

#[derive(Queryable)]
pub struct Entry {
    pub id: String,
    pub feed_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub defunct: bool,
    pub json: String,
    pub title: String,
    pub link: String,
    pub summary: String,
    pub content: String,
}

#[derive(Insertable)]
#[table_name = "entries"]
pub struct EntryNew<'a> {
    pub id: &'a str,
    pub feed_id: &'a str,
    pub published: &'a str,
    pub created_at: &'a str,
    pub updated_at: &'a str,
    pub defunct: bool,
    pub title: &'a str,
    pub link: &'a str,
    pub summary: &'a str,
    pub content: &'a str,
}

#[derive(AsChangeset)]
#[table_name = "entries"]
pub struct EntryUpdate<'a> {
    pub published: Option<&'a str>,
    pub updated_at: Option<&'a str>,
    pub defunct: Option<bool>,
    pub title: Option<&'a str>,
    pub link: Option<&'a str>,
    pub summary: Option<&'a str>,
    pub content: Option<&'a str>,
}

#[derive(Queryable)]
pub struct FeedHistory {
    pub id: String,
    pub feed_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub src: String,
    pub status: String,
    pub status_text: String,
}

#[derive(Insertable)]
#[table_name = "feed_history"]
pub struct FeedHistoryNew<'a> {
    pub id: &'a str,
    pub feed_id: &'a str,
    pub created_at: &'a str,
    pub src: &'a str,
    pub status: &'a str,
}

#[derive(Queryable)]
pub struct Feed {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
    pub title: String,
    pub subtitle: String,
    pub link: String,
    pub json: String,
}

#[derive(Insertable)]
#[table_name = "feeds"]
pub struct FeedNew<'a> {
    pub id: &'a str,
    pub published: &'a str,
    pub created_at: &'a str,
    pub updated_at: &'a str,
    pub url: &'a str,
    pub title: &'a str,
    pub link: &'a str,
}

#[derive(AsChangeset)]
#[table_name = "feeds"]
pub struct FeedUpdate<'a> {
    pub published: Option<&'a str>,
    pub updated_at: Option<&'a str>,
    pub url: Option<&'a str>,
    pub title: Option<&'a str>,
    pub link: Option<&'a str>,
}
