//! Backend-neutral wire types. No search-engine types cross this boundary.
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("Search storage failed: {0}")]
    Storage(String),
    #[error("The index changed. Start a new search.")]
    StaleCursor,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub tweet_id: String,
    pub author: String,
    pub author_id: String,
    pub text: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub likes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reposts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replies: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quotes: Option<u32>,
    pub links: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
    #[default]
    Relevance,
    Engagement,
    Likes,
    Newest,
    Oldest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    pub version: u8,
    pub query: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub sort: Sort,
    pub limit: usize,
    #[serde(default)]
    pub cursor: Option<String>,
}

impl SearchRequest {
    /// Validate the transport bounds before parsing or searching.
    ///
    /// # Errors
    /// Returns an error for unsupported versions or excessive work.
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 || !(1..=20).contains(&self.limit) {
            return Err(Error::Invalid("Use version 1 and limit 1–20.".into()));
        }
        if self.query.chars().count() > 300 || self.cursor.as_ref().is_some_and(|s| s.len() > 4000)
        {
            return Err(Error::Invalid("Query or cursor is too long.".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub rows: Vec<Post>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub warnings: Vec<String>,
}
