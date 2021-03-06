use super::Context;
use crate::db::paginate_dsl::{PaginateDsl, Pagination};
use crate::models;
use chrono::prelude::*;
use diesel::prelude::*;
use juniper::{graphql_object, FieldResult};

#[allow(clippy::module_name_repetitions)]
pub struct RootQuery;

#[graphql_object(context = Context)]
impl RootQuery {
    fn apiVersion() -> &str {
        "1.0"
    }

    // TODO: figure out how to parameterize this paginate / since / order code and move into the db module
    fn feeds(
        context: &Context,
        since: Option<DateTime<Utc>>,
        pagination: Option<Pagination>,
    ) -> FieldResult<Vec<models::Feed>> {
        use crate::schema::feeds::dsl::{feeds, last_entry_published};
        let conn = context.pool.get()?;
        let mut query = feeds.into_boxed();
        if let Some(since) = since {
            query = query.filter(last_entry_published.gt(since.to_rfc3339()));
        }
        query = query
            .paginate(pagination)
            .order(last_entry_published.desc());
        Ok(query.load::<models::Feed>(&conn)?)
    }

    fn entries(
        context: &Context,
        since: Option<DateTime<Utc>>,
        pagination: Option<Pagination>,
    ) -> FieldResult<Vec<models::Entry>> {
        use crate::schema::entries::dsl::{entries, published};
        let conn = context.pool.get()?;
        let mut query = entries.into_boxed();
        if let Some(since) = since {
            query = query.filter(published.gt(since.to_rfc3339()));
        }
        query = query.paginate(pagination).order(published.desc());
        Ok(query.load::<models::Entry>(&conn)?)
    }
}

#[graphql_object(
    description = "An entry in a syndication feed",
    context = Context,
)]
impl models::Entry {
    fn id(&self) -> &Option<String> {
        &self.id
    }
    fn feed_id(&self) -> &Option<String> {
        &self.feed_id
    }
    fn published(&self) -> &Option<String> {
        &self.published
    }
    fn created_at(&self) -> &Option<String> {
        &self.created_at
    }
    fn modified_at(&self) -> &Option<String> {
        &self.modified_at
    }
    fn defunct(&self) -> &Option<bool> {
        &self.defunct
    }
    fn json(&self) -> &Option<String> {
        &self.json
    }
    fn guid(&self) -> &Option<String> {
        &self.guid
    }
    fn title(&self) -> &Option<String> {
        &self.title
    }
    fn link(&self) -> &Option<String> {
        &self.link
    }
    fn summary(&self) -> &Option<String> {
        &self.summary
    }
    fn content(&self) -> &Option<String> {
        &self.content
    }
    fn updated(&self) -> &Option<String> {
        &self.updated
    }
    fn feed(&self, context: &Context) -> FieldResult<models::Feed> {
        use crate::schema::feeds::dsl::{feeds, id};
        let conn = context.pool.get()?;
        Ok(feeds
            .filter(id.eq(&self.feed_id))
            .first::<models::Feed>(&conn)?)
    }
}

#[graphql_object(
    description = "A syndication feed",
    context = Context,
)]
impl models::Feed {
    fn id(&self) -> &Option<String> {
        &self.id
    }
    fn published(&self) -> &Option<String> {
        &self.published
    }
    fn created_at(&self) -> &Option<String> {
        &self.created_at
    }
    fn modified_at(&self) -> &Option<String> {
        &self.modified_at
    }
    fn url(&self) -> &Option<String> {
        &self.url
    }
    fn title(&self) -> &Option<String> {
        &self.title
    }
    fn subtitle(&self) -> &Option<String> {
        &self.subtitle
    }
    fn link(&self) -> &Option<String> {
        &self.link
    }
    fn json(&self) -> &Option<String> {
        &self.json
    }
    fn updated(&self) -> &Option<String> {
        &self.updated
    }
    fn last_entry_published(&self) -> &Option<String> {
        &self.last_entry_published
    }
    // TODO: is there any way to optimize this as a LEFT JOIN? check out juniper look-ahead
    fn entries(
        &self,
        context: &Context,
        since: Option<DateTime<Utc>>,
        pagination: Option<Pagination>,
    ) -> FieldResult<Vec<models::Entry>> {
        use crate::schema::entries::dsl::{entries, feed_id, published};
        let conn = context.pool.get()?;
        let mut query = entries.into_boxed();
        query = query.filter(feed_id.eq(&self.id));
        if let Some(since) = since {
            query = query.filter(published.gt(since.to_rfc3339()));
        }
        query = query.paginate(pagination).order(published.desc());
        Ok(query.load::<models::Entry>(&conn)?)
    }
    fn history(
        &self,
        context: &Context,
        since: Option<DateTime<Utc>>,
        pagination: Option<Pagination>,
    ) -> FieldResult<Vec<models::FeedHistory>> {
        use crate::schema::feed_history::dsl::{created_at, feed_history, feed_id};
        let conn = context.pool.get()?;
        let mut query = feed_history.into_boxed();
        query = query.filter(feed_id.eq(&self.id));
        if let Some(since) = since {
            query = query.filter(created_at.gt(since.to_rfc3339()));
        }
        query = query.paginate(pagination).order(created_at.desc());
        Ok(query.load::<models::FeedHistory>(&conn)?)
    }
}
