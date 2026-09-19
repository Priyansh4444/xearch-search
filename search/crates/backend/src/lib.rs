//! The only boundary an alternative retrieval engine needs to implement.
use search_model::{Post, Result, SearchRequest, SearchResponse};
use search_query::Expr;

pub trait SearchBackend: Send + Sync {
    /// Retrieve one bounded page, applying the complete expression before ranking.
    ///
    /// # Errors
    /// Returns validation, stale-cursor or storage errors. Never silently relaxes a query.
    fn search(
        &self,
        expression: &Expr,
        request: &SearchRequest,
        now: i64,
    ) -> Result<SearchResponse>;
}

pub trait IndexSink {
    /// Stage an idempotent replacement by source tweet ID.
    ///
    /// # Errors
    /// Returns storage or document validation errors.
    fn upsert(&mut self, post: &Post) -> Result<()>;

    /// Make all staged documents durable and visible together.
    ///
    /// # Errors
    /// Returns storage errors; callers must not record import success on failure.
    fn commit(&mut self) -> Result<()>;
}
