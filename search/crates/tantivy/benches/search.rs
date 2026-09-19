use criterion::Criterion;
use search_backend::{IndexSink, SearchBackend};
use std::hint::black_box;
#[path = "../tests/support/mod.rs"]
mod support;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut c = Criterion::default().configure_from_args();
    let dir = tempfile::tempdir()?;
    let engine = search_tantivy::open(dir.path(), true)?;
    let mut writer = engine.writer()?;
    for id in 1..=2000 {
        let text = if id % 10 == 0 {
            "rust search on disk to be or not to be"
        } else {
            "the and a to be fast storage"
        };
        writer.upsert(&support::post(id, text))?;
    }
    writer.commit()?;
    for query in [
        "rust disk",
        "the",
        "\"to be or not to be\"",
        "from:alice",
        "the -rust",
    ] {
        let req = support::request(query);
        let expression = search_query::parse(query, None)?;
        let mut failure = None;
        c.bench_function(query, |b| {
            b.iter(
                || match black_box(engine.search(&expression, &req, 1_800_000_000_000)) {
                    Ok(response) => {
                        black_box(response);
                    }
                    Err(error) => failure = Some(error),
                },
            );
        });
        if let Some(error) = failure {
            return Err(error.into());
        }
    }
    c.final_summary();
    drop(c);
    Ok(())
}
