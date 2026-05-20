use std::path::PathBuf;

use crate::models::branch::Branch;
use crate::models::commit::Commit;
use crate::models::repository::Repository;
use crate::models::status::WorkingDirectoryStatus;

#[derive(Debug, Clone)]
pub enum Message {
    // ── Repository lifecycle ──────────────────────────────────────────────
    LoadRepository(PathBuf),
    Refresh,
    DataLoaded(Result<GitSnapshot, String>),

    // ── Working-directory file selection ─────────────────────────────────
    ToggleAllFiles(bool),
    ToggleFile(String, bool),

    // ── Branch operations ─────────────────────────────────────────────────
    BranchSelected(String),
    CheckoutCompleted(Result<String, String>),

    // ── Commit panel ──────────────────────────────────────────────────────
    /// The user edited the one-line summary field.
    CommitSummaryChanged(String),
    /// The user edited the multi-line description field.
    CommitDescriptionChanged(String),
    /// The user pressed the "Commit to <branch>" button.
    Commit,
    /// Async result from `GitEngine::create_commit`.
    /// `Ok` carries the new commit SHA as a hex string.
    CommitCompleted(Result<String, String>),
}

#[derive(Debug, Clone)]
pub struct GitSnapshot {
    pub repository: Repository,
    pub status: WorkingDirectoryStatus,
    pub commits: Vec<Commit>,
    pub branches: Vec<Branch>,
}
