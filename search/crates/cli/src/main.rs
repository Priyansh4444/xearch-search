use clap::{Parser, Subcommand};
use search_backend::SearchBackend;
use search_model::{SearchRequest, Sort};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};

#[derive(Parser)]
#[command(about = "Disk-backed X search. Imports and postings stay on this machine.")]
struct Cli {
    #[arg(long, env = "SEARCH_INDEX")]
    index: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Retain and import an x.md JSON dump or capture. Never calls a provider.
    Import {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        archive: PathBuf,
    },
    /// Run one search and print the version-1 JSON response.
    Query {
        query: String,
        #[arg(long, default_value = "relevance", value_parser = ["relevance", "engagement", "likes", "newest", "oldest"])]
        sort: String,
    },
    /// Serve the existing app contract on loopback.
    Serve {
        #[arg(long, default_value = "127.0.0.1:4320")]
        listen: SocketAddr,
    },
}

#[tokio::main(worker_threads = 2)]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    match cli.command {
        Command::Import { input, archive } => {
            let engine = search_tantivy::open(&cli.index, true)?;
            let mut writer = engine.writer()?;
            let receipt = search_ingest::import(&input, &archive, &mut writer)?;
            println!("{}", serde_json::to_string(&receipt)?);
        }
        Command::Query { query, sort } => {
            let engine = search_tantivy::open(&cli.index, false)?;
            let sort: Sort = serde_json::from_value(serde_json::Value::String(sort))?;
            let expression = search_query::parse(&query, None)?;
            let request = SearchRequest {
                version: 1,
                query,
                author: None,
                sort,
                limit: 20,
                cursor: None,
            };
            println!(
                "{}",
                serde_json::to_string(&engine.search(
                    &expression,
                    &request,
                    jiff::Timestamp::now().as_millisecond()
                )?)?
            );
        }
        Command::Serve { listen } => {
            color_eyre::eyre::ensure!(
                listen.ip().is_loopback(),
                "Bind loopback; expose only through the authenticated proxy."
            );
            let key = std::env::var("SEARCH_LOCAL_SIGNING_KEY")?.into_bytes();
            let bearer = std::env::var("SEARCH_SERVICE_TOKEN")?.into_bytes();
            let engine = Arc::new(search_tantivy::open(&cli.index, false)?);
            let app = search_api::router(engine, key, bearer)?;
            let listener = tokio::net::TcpListener::bind(listen).await?;
            eprintln!("Search listening on {listen}");
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
        }
    }
    Ok(())
}
