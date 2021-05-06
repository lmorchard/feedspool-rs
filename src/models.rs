use super::schema::feeds;

#[derive(Queryable)]
pub struct Entry {
  pub id: String,
  pub feed_id: String,
  pub created_at: String,
  pub updated_at: String,
  pub defunct: bool,
  pub json: String,
  pub guid: String,
  pub title: String,
  pub link: String,
  pub summary: String,
  pub content: String,
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

#[derive(Queryable)]
pub struct Feed {
  pub id: String,
  pub created_at: String,
  pub updated_at: String,
  pub url: String,
  pub title: String,
  pub subtitle: String,
  pub link: String,
}

#[derive(Insertable)]
#[table_name="feeds"]
pub struct NewFeed<'a> {
    pub id: &'a str,
    pub created_at: &'a str,
    pub updated_at: &'a str,
    pub url: &'a str,
    pub title: &'a str,
    pub link: &'a str,
}
