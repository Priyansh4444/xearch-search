//! Ranking signals adapted from the old Convex engine, independent of retrieval.
use search_model::Post;

/// Weighted engagement, with missing counts contributing zero.
#[must_use]
pub fn engagement(post: &Post) -> f64 {
    f64::from(post.quotes.unwrap_or(0))
        .mul_add(
            4.0,
            f64::from(post.reposts.unwrap_or(0)).mul_add(
                3.0,
                f64::from(post.replies.unwrap_or(0))
                    .mul_add(2.0, f64::from(post.likes.unwrap_or(0))),
            ),
        )
        .ln_1p()
}

/// Bounded engagement boost. Recency is optional when the source lacks a date.
#[must_use]
pub fn blended(relevance: f32, engagement: f64, created_at: Option<i64>, now: i64) -> f64 {
    let relevance = f64::from(relevance.max(0.0));
    let recency = created_at.map_or(0.0, |created| {
        let age = u32::try_from(now.saturating_sub(created).max(0) / 1000).unwrap_or(u32::MAX);
        (-f64::from(age) / 172_800.0).exp()
    });
    0.15_f64.mul_add(
        recency,
        (0.35 * relevance / (1.0 + relevance)) + (0.20 * engagement / (1.0 + engagement)),
    )
}
