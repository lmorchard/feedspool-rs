use juniper::EmptySubscription;

// TODO: should gql be a feature? just a part of the binary and not library?
pub mod mutation;
pub mod query;

use crate::db::SqlitePool;
use crate::gql::mutation::RootMutation;
use crate::gql::query::RootQuery;

pub struct Context {
    pub pool: SqlitePool,
}

impl juniper::Context for Context {}

// A root schema consists of a query, a mutation, and a subscription.
// Request queries can be executed against a RootNode.
pub type Schema = juniper::RootNode<'static, RootQuery, RootMutation, EmptySubscription<Context>>;
