use search_backend::SearchBackend;
use search_model::{SearchRequest, Sort};
use std::io::Write;

#[test]
fn retained_import_replays_and_quarantines_invalid_records() {
    let root = tempfile::tempdir().unwrap();
    let mut input = tempfile::NamedTempFile::new().unwrap();
    let good = serde_json::json!({"id":"123", "text":"to be", "author":{"id":"7", "screen_name":"Alice"}, "likes":0, "created_timestamp":1_700_000_000});
    write!(
        input,
        "{}",
        serde_json::json!({"posts":[good, {"id":"bad"}]})
    )
    .unwrap();
    let engine = search_tantivy::open(&root.path().join("index"), true).unwrap();
    let mut writer = engine.writer().unwrap();
    let archive = root.path().join("raw");
    for _ in 0..2 {
        let receipt = search_ingest::import(input.path(), &archive, &mut writer).unwrap();
        assert_eq!((receipt.accepted, receipt.rejected), (1, 1));
        assert_eq!(
            std::fs::read(archive.join(format!("{}.json", receipt.sha256))).unwrap(),
            std::fs::read(input.path()).unwrap()
        );
    }
    let request = SearchRequest {
        version: 1,
        query: "be".into(),
        author: None,
        sort: Sort::Relevance,
        limit: 20,
        cursor: None,
    };
    let result = engine
        .search(
            &search_query::parse("be", None).unwrap(),
            &request,
            1_700_000_000_000,
        )
        .unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].created_at, Some(1_700_000_000_000));
    assert_eq!(result.rows[0].replies, None);
}

#[test]
fn malformed_envelope_does_not_publish_partial_index() {
    let root = tempfile::tempdir().unwrap();
    let mut input = tempfile::NamedTempFile::new().unwrap();
    write!(input, "{{\"posts\":[{{\"id\":\"1\",\"text\":\"hello\",\"author\":{{\"id\":\"2\",\"screen_name\":\"alice\"}}}}],BROKEN").unwrap();
    let engine = search_tantivy::open(&root.path().join("index"), true).unwrap();
    {
        let mut writer = engine.writer().unwrap();
        assert!(
            search_ingest::import(input.path(), &root.path().join("raw"), &mut writer).is_err()
        );
    }
    let request = SearchRequest {
        version: 1,
        query: "hello".into(),
        author: None,
        sort: Sort::Relevance,
        limit: 20,
        cursor: None,
    };
    assert!(
        engine
            .search(&search_query::parse("hello", None).unwrap(), &request, 0)
            .unwrap()
            .rows
            .is_empty()
    );
}
