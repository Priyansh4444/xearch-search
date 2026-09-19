use search_model::{Post, SearchRequest, Sort};

pub fn post(id: u32, text: &str) -> Post {
    Post {
        tweet_id: id.to_string(),
        author: "alice".into(),
        author_id: "100".into(),
        text: text.into(),
        url: format!("https://x.com/alice/status/{id}"),
        created_at: Some(i64::from(id).saturating_mul(1000)),
        likes: Some(id),
        reposts: None,
        replies: None,
        quotes: None,
        links: Vec::new(),
        display_name: None,
        avatar: None,
    }
}

pub fn request(query: &str) -> SearchRequest {
    SearchRequest {
        version: 1,
        query: query.into(),
        author: None,
        sort: Sort::Relevance,
        limit: 20,
        cursor: None,
    }
}
