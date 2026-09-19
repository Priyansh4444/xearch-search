//! Per-user ingestion registry: one file marks every intake account as
//! complete, incomplete, or error, so postings can be built per user and
//! failures retried without guessing.
//!
//! Layout: the state directory holds `users.json`:
//! ```json
//! {
//!   "version": 1,
//!   "users": {
//!     "somehandle": {
//!       "status": "complete",
//!       "attempts": 2,
//!       "accepted": 150,
//!       "rejected": 0,
//!       "sha256": "ab12…",
//!       "lastError": null,
//!       "fileSig": "52341:1758000000000",
//!       "updatedAtMs": 1758000000000
//!     }
//!   }
//! }
//! ```
//!
//! Transition rules (applied by [`crate::run_once`]):
//! - New intake file: `incomplete`, zero attempts.
//! - Import accepting ≥1 post: `complete` with receipt details.
//! - Import accepting 0 posts or failing: `error` with reason, attempts + 1.
//! - `error` / `incomplete` users are retried every pass; `complete` users
//!   are skipped unless their file changes (or an operator marks them
//!   `incomplete`, which clears the file signature and forces reimport).

use search_ingest::Receipt;
use search_model::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Filename of the registry inside the state directory.
pub const REGISTRY_FILE: &str = "users.json";

/// Ingestion state of one intake account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    /// Last import accepted at least one post.
    Complete,
    /// Seen but never successfully imported; pending or retryable.
    Incomplete,
    /// Last attempt failed; see `last_error` on the record.
    Error,
}

impl std::str::FromStr for UserStatus {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        match text {
            "complete" => Ok(Self::Complete),
            "incomplete" => Ok(Self::Incomplete),
            "error" => Ok(Self::Error),
            _ => Err(Error::Invalid(
                "Status must be complete, incomplete, or error.".into(),
            )),
        }
    }
}

/// Retry and receipt history for one intake account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRecord {
    pub status: UserStatus,
    /// Total import attempts so far.
    pub attempts: u32,
    /// Posts accepted by the last successful import.
    pub accepted: u64,
    /// Records quarantined by the last successful import.
    pub rejected: u64,
    /// Content hash of the last successfully imported file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Reason for the last failure; `None` while complete.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    /// `size:mtime` of the last imported file; cleared to force reimport.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_sig: Option<String>,
    pub updated_at_ms: i64,
}

impl UserRecord {
    const fn fresh(now: i64) -> Self {
        Self {
            status: UserStatus::Incomplete,
            attempts: 0,
            accepted: 0,
            rejected: 0,
            sha256: None,
            last_error: None,
            file_sig: None,
            updated_at_ms: now,
        }
    }
}

/// The registry file: versioned map of handle to record.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Registry {
    pub version: u8,
    #[serde(default)]
    pub users: HashMap<String, UserRecord>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

fn storage(error: impl std::fmt::Display) -> Error {
    Error::Storage(error.to_string())
}

impl Registry {
    /// Load the registry; a missing file is an empty registry, not an error.
    ///
    /// # Errors
    /// Returns storage errors for unreadable or malformed files.
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self {
                version: 1,
                users: HashMap::new(),
            }),
            Err(error) => Err(storage(error)),
            Ok(bytes) => {
                let registry: Self =
                    serde_json::from_slice(&bytes).map_err(|e| Error::Invalid(e.to_string()))?;
                if registry.version != 1 {
                    return Err(Error::Invalid("Unsupported users registry version.".into()));
                }
                Ok(registry)
            }
        }
    }

    /// Persist atomically (temp file + rename) so a crash never leaves a
    /// half-written registry behind.
    ///
    /// # Errors
    /// Returns storage errors when the state directory cannot be written.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(storage)?;
        }
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(storage)?;
        serde_json::to_writer_pretty(&mut tmp, self).map_err(storage)?;
        tmp.flush().map_err(storage)?;
        tmp.as_file().sync_all().map_err(storage)?;
        tmp.persist(path).map_err(storage)?;
        Ok(())
    }

    /// Fetch the record for a handle, inserting a fresh `incomplete` one.
    pub fn record(&mut self, handle: &str) -> &mut UserRecord {
        self.users
            .entry(handle.to_owned())
            .or_insert_with(|| UserRecord::fresh(now_ms()))
    }

    /// Mark a successful import. Zero accepted posts is an error, not a
    /// completion, so empty retries stay visible.
    pub fn mark_complete(&mut self, handle: &str, receipt: &Receipt, file_sig: &str) {
        let now = now_ms();
        let record = self.record(handle);
        record.attempts = record.attempts.saturating_add(1);
        if receipt.accepted == 0 {
            record.status = UserStatus::Error;
            record.last_error = Some(format!("No posts accepted; {} rejected.", receipt.rejected));
        } else {
            record.status = UserStatus::Complete;
            record.accepted = receipt.accepted;
            record.rejected = receipt.rejected;
            record.sha256 = Some(receipt.sha256.clone());
            record.last_error = None;
        }
        record.file_sig = Some(file_sig.to_owned());
        record.updated_at_ms = now;
    }

    /// Mark a failed attempt with its reason; attempts accumulate for backoff
    /// and triage. The file signature is left alone so the next pass retries
    /// the same bytes.
    pub fn mark_error(&mut self, handle: &str, reason: &str) {
        let now = now_ms();
        let record = self.record(handle);
        record.attempts = record.attempts.saturating_add(1);
        record.status = UserStatus::Error;
        record.last_error = Some(reason.to_owned());
        record.updated_at_ms = now;
    }

    /// Operator override: set a status by hand. Marking `incomplete` clears
    /// the file signature so the next pass reimports even unchanged bytes;
    /// marking `complete` keeps history but requires a note explaining why.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] for a manual `complete` without a note.
    pub fn mark(&mut self, handle: &str, status: UserStatus, note: Option<&str>) -> Result<()> {
        if status == UserStatus::Complete && note.is_none_or(str::is_empty) {
            return Err(Error::Invalid(
                "Marking complete by hand requires --note (sha or reason).".into(),
            ));
        }
        let now = now_ms();
        let record = self.record(handle);
        record.status = status;
        match status {
            UserStatus::Incomplete => {
                record.file_sig = None;
                record.last_error = note.map(str::to_owned);
            }
            UserStatus::Error => {
                record.last_error = Some(note.unwrap_or("Marked error by operator.").to_owned());
            }
            UserStatus::Complete => {
                record.last_error = None;
            }
        }
        record.updated_at_ms = now;
        Ok(())
    }
}

/// Registry path for a state directory.
#[must_use]
pub fn registry_path(state_dir: &Path) -> PathBuf {
    state_dir.join(REGISTRY_FILE)
}
