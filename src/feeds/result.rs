#![allow(clippy::module_name_repetitions)]

use feed_rs::model::Feed;
use std::error::Error;
use std::fmt;
use std::panic;

#[derive(Debug)]
pub struct ConditionalGetData {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug)]
pub struct FeedFetchResult {
    pub id: String,
    pub url: String,
    pub status: String,
    pub headers: reqwest::header::HeaderMap,
    pub body: String,
}

#[derive(Debug)]
pub enum FeedPollResult {
    Skipped,
    NotModified { fetch: FeedFetchResult },
    Fetched { fetch: FeedFetchResult, feed: Feed },
    Updated { fetch: FeedFetchResult, feed: Feed },
}

impl FeedPollResult {
    /// # Panics
    ///
    /// Will panic if attempted on something other than `FeedPollResult::Fetched`
    #[must_use]
    pub fn fetched_to_updated(self) -> FeedPollResult {
        match self {
            Self::Fetched { fetch, feed } => Self::Updated { fetch, feed },
            _ => panic!("expected FeedPollResult::Fetched, got {:?}", self),
        }
    }

    /// # Panics
    ///
    /// Will panic if attempted on something other than `FeedPollResult::Fetched`
    #[must_use]
    pub fn fetched_to_update_error(self, error: diesel::result::Error) -> FeedPollError {
        match self {
            Self::Fetched { fetch, feed } => FeedPollError::UpdateError { fetch, feed, error },
            _ => panic!("expected FeedPollResult::Fetched, got {:?}", self),
        }
    }
}

#[derive(Debug)]
pub enum FeedPollError {
    FetchTimeError(time::OutOfRangeError),
    Timedout(reqwest::Error),
    NotFound(reqwest::Error),
    FetchError(reqwest::Error),
    DatabaseError(diesel::result::Error),
    FetchFailed {
        fetch: FeedFetchResult,
    },
    ParseError {
        fetch: FeedFetchResult,
        error: feed_rs::parser::ParseFeedError,
    },
    UpdateError {
        fetch: FeedFetchResult,
        feed: Feed,
        error: diesel::result::Error,
    },
}
impl FeedPollError {}
impl fmt::Display for FeedPollError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}
impl Error for FeedPollError {}
