//! Background indexer: poll a drop directory and import per-user dumps.
//!
//! Each intake file is named `<handle>.json` (or `.jsonl`) and feeds one
//! entry in `state_dir/users.json` (`see users`), marked complete,
//! incomplete, or error. Imports stay idempotent:
//! [`search_ingest::import`] retains exact input bytes under a
//! content-addressed archive name and converges on reimport, so a crash
//! between passes is safe to replay. A single writer lock owned per pass
//! serializes with one-shot `import` invocations.
//!
//! All directories come from the caller (CLI flags or `SEARCH_*`
//! environment); nothing here assumes which machine it runs on.

pub mod users;

use search_model::{Error, Result};
use sha2::Digest;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use users::{Registry, registry_path};

/// What to watch and where postings live. Every path is caller-supplied so
/// the same binary runs on any machine via environment.
#[derive(Debug, Clone)]
pub struct Config {
    /// Tantivy index directory (created on first pass).
    pub index: PathBuf,
    /// Content-addressed archive directory.
    pub archive: PathBuf,
    /// Polled drop directory holding per-user `<handle>.json[.l]` dumps.
    pub drop_dir: PathBuf,
    /// Directory holding `users.json`.
    pub state_dir: PathBuf,
    /// Delay between passes.
    pub poll_interval: Duration,
}

impl Config {
    /// Validate bounds before watching.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] for an empty path or a zero poll interval.
    pub fn validate(&self) -> Result<()> {
        if self.index.as_os_str().is_empty()
            || self.archive.as_os_str().is_empty()
            || self.drop_dir.as_os_str().is_empty()
            || self.state_dir.as_os_str().is_empty()
        {
            return Err(Error::Invalid(
                "Index, archive, drop and state paths are required.".into(),
            ));
        }
        if self.poll_interval.is_zero() {
            return Err(Error::Invalid(
                "Poll interval must be greater than zero.".into(),
            ));
        }
        Ok(())
    }
}

/// Content-hash signature. Unlike size:mtime, a same-size edit with a
/// restored or coarse mtime can never pass as unchanged.
fn signature(file: &Path) -> Option<String> {
    let bytes = std::fs::read(file).ok()?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&bytes);
    Some(format!("{:x}", hasher.finalize()))
}

fn candidates(drop_dir: &Path) -> Vec<PathBuf> {
    let entries = std::fs::read_dir(drop_dir).map_or_else(|_| Vec::new(), Iterator::collect);
    let mut files: Vec<PathBuf> = entries
        .into_iter()
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "json" || ext == "jsonl")
        })
        .filter(|path| std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file()))
        .collect();
    files.sort();
    files
}

/// Handle for an intake file: the filename stem, normalized like query
/// authors (`@` stripped, lowercased). Returns `None` when the stem is not
/// a usable handle.
fn handle_for(file: &Path) -> Option<String> {
    let stem = file.file_stem()?.to_str()?;
    search_query::normalize_author(stem).ok()
}

/// Import every due dump once, updating the registry.
///
/// A user is due when its status is not `complete`, or when the bytes of
/// its recorded file changed. Files whose handle already belongs to a
/// different path are skipped with a warning until the operator removes the
/// collision — two files fighting over one registry entry would otherwise
/// reimport against each other every pass. Returns the registry.
///
/// # Errors
/// Returns [`Error::Storage`] if the index cannot be opened or the registry
/// cannot be loaded or saved.
pub fn run_once(config: &Config) -> Result<Registry> {
    let path = registry_path(&config.state_dir);
    let mut registry = Registry::load(&path)?;
    let engine = search_tantivy::open(&config.index, true)?;
    for file in candidates(&config.drop_dir) {
        let Some(handle) = handle_for(&file) else {
            eprintln!("indexer skip file={} unusable handle", file.display());
            continue;
        };
        let Some(file_name) = file.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };
        let conflicted = {
            let record = registry.record(&handle);
            record
                .file_name
                .as_deref()
                .is_some_and(|existing| existing != file_name)
        };
        if conflicted {
            eprintln!(
                "indexer skip user={handle}: {} collides with {}",
                file_name,
                registry.record(&handle).file_name.as_deref().unwrap_or("?")
            );
            continue;
        }
        let Some(sig) = signature(&file) else {
            eprintln!("indexer skip file={} unreadable", file.display());
            continue;
        };
        let due = {
            let record = registry.record(&handle);
            record.status != users::UserStatus::Complete || record.file_sig.as_deref() != Some(&sig)
        };
        if !due {
            continue;
        }
        let writer = match engine.writer() {
            Ok(writer) => writer,
            Err(error) => {
                registry.mark_error(&handle, &error.to_string(), Some(&file_name));
                eprintln!("indexer err user={handle} {error}");
                registry.save(&path)?;
                continue;
            }
        };
        match search_ingest::import(&file, &config.archive, writer) {
            Ok(receipt) => {
                registry.mark_complete(&handle, &receipt, &sig, &file_name);
                eprintln!(
                    "indexer ok user={handle} accepted={} rejected={}",
                    receipt.accepted, receipt.rejected
                );
            }
            Err(error) => {
                registry.mark_error(&handle, &error.to_string(), Some(&file_name));
                eprintln!("indexer err user={handle} {error}");
            }
        }
        registry.save(&path)?;
    }
    registry.save(&path)?;
    Ok(registry)
}

/// Poll forever until Ctrl-C. Each pass loads the registry, imports due
/// users, and saves. Never calls a provider; only local files are read.
///
/// # Errors
/// Returns validation errors before the first pass.
pub async fn watch(config: Config) -> Result<()> {
    config.validate()?;
    // Immediate first pass so restarts pick up waiting dumps at once.
    if let Err(error) = run_once(&config) {
        eprintln!("indexer pass failed: {error}");
    }
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                eprintln!("indexer stopping");
                return Ok(());
            }
            () = tokio::time::sleep(config.poll_interval) => {
                if let Err(error) = run_once(&config) {
                    eprintln!("indexer pass failed: {error}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use users::UserStatus;

    fn config_in(dir: &Path) -> Config {
        let config = Config {
            index: dir.join("index"),
            archive: dir.join("archive"),
            drop_dir: dir.join("drop"),
            state_dir: dir.join("state"),
            poll_interval: Duration::from_secs(1),
        };
        std::fs::create_dir_all(&config.drop_dir).expect("drop dir");
        config
    }

    fn post(id: &str, handle: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "author": {"screen_name": handle, "id": "42"},
            "text": "hello world",
            "created_timestamp": 1_758_000_000_i64,
        })
    }

    #[test]
    fn rejects_zero_interval() {
        let config = Config {
            index: PathBuf::from("i"),
            archive: PathBuf::from("a"),
            drop_dir: PathBuf::from("d"),
            state_dir: PathBuf::from("s"),
            poll_interval: Duration::ZERO,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn empty_drop_dir_writes_empty_registry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = config_in(dir.path());
        let registry = run_once(&config).expect("run once");
        assert!(registry.users.is_empty());
        assert!(registry_path(&config.state_dir).exists());
    }

    #[test]
    fn per_user_file_marks_complete_and_skips_second_pass() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = config_in(dir.path());
        std::fs::write(
            config.drop_dir.join("TestUser.json"),
            serde_json::to_string(&serde_json::json!({"posts": [post("1001", "TestUser")] }))
                .expect("json"),
        )
        .expect("write dump");
        let registry = run_once(&config).expect("first pass");
        let record = registry.users.get("testuser").expect("record");
        assert_eq!(record.status, UserStatus::Complete);
        assert_eq!(record.attempts, 1);
        assert_eq!(record.accepted, 1);

        let registry = run_once(&config).expect("second pass");
        let record = registry.users.get("testuser").expect("record");
        assert_eq!(record.attempts, 1, "unchanged file must not reimport");
    }

    #[test]
    fn malformed_dump_marks_error_and_retries() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = config_in(dir.path());
        std::fs::write(config.drop_dir.join("baduser.json"), "{not json").expect("write dump");
        let registry = run_once(&config).expect("first pass");
        let record = registry.users.get("baduser").expect("record");
        assert_eq!(record.status, UserStatus::Error);
        assert!(record.last_error.is_some());

        let registry = run_once(&config).expect("second pass");
        let record = registry.users.get("baduser").expect("record");
        assert_eq!(record.attempts, 2, "error users retry every pass");
    }

    #[test]
    fn manual_mark_incomplete_forces_reimport() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = config_in(dir.path());
        std::fs::write(
            config.drop_dir.join("u.json"),
            serde_json::to_string(&serde_json::json!({"posts": [post("2001", "u")] }))
                .expect("json"),
        )
        .expect("write dump");
        run_once(&config).expect("first pass");
        let path = registry_path(&config.state_dir);
        let mut registry = Registry::load(&path).expect("load");
        registry
            .mark("u", UserStatus::Incomplete, Some("recheck"))
            .expect("mark");
        registry.save(&path).expect("save");
        let registry = run_once(&config).expect("second pass");
        assert_eq!(
            registry.users.get("u").expect("record").attempts,
            2,
            "cleared signature must reimport"
        );
    }

    #[test]
    fn same_size_edit_reimports_via_content_hash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = config_in(dir.path());
        let path = config.drop_dir.join("edituser.json");
        std::fs::write(&path, r#"{"posts":[]}"#).expect("write dump");
        // Same-length edit inside the same clock tick would defeat mtime.
        let first = r#"{"posts":[{"id":"3001","author":{"screen_name":"edituser","id":"7"},"text":"alpha","created_timestamp":1758000000}]}"#;
        std::fs::write(&path, first).expect("write v1");
        run_once(&config).expect("first pass");
        let second = first.replace("alpha", "omega");
        assert_eq!(first.len(), second.len(), "same-size precondition");
        std::fs::write(&path, second).expect("write v2");
        let registry = run_once(&config).expect("second pass");
        let record = registry.users.get("edituser").expect("record");
        assert_eq!(
            record.status,
            UserStatus::Complete,
            "changed bytes must reimport even at same size"
        );
        assert_eq!(record.accepted, 1);
        assert_eq!(record.attempts, 2);
    }

    #[test]
    fn colliding_handles_skip_instead_of_oscillate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = config_in(dir.path());
        for (name, id) in [("Alice.json", "4001"), ("alice.json", "4002")] {
            std::fs::write(
                config.drop_dir.join(name),
                serde_json::to_string(&serde_json::json!({"posts": [post(id, "Alice")] }))
                    .expect("json"),
            )
            .expect("write dump");
        }
        for pass in 1..=3 {
            let registry = run_once(&config).expect("pass");
            let record = registry.users.get("alice").expect("record");
            assert_eq!(
                record.attempts, 1,
                "collision must not reimport on pass {pass}"
            );
        }
    }
}
