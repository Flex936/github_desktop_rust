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
    CommitSummaryChanged(String),
    CommitDescriptionChanged(String),
    Commit,
    CommitCompleted(Result<String, String>),

    // ── Diff view ─────────────────────────────────────────────────────────
    /// The user clicked a file row in the sidebar.
    FileClicked(PathBuf),
    /// Async result from `GitEngine::get_file_diff`.
    DiffLoaded(Result<String, String>),
}

#[derive(Debug, Clone)]
pub struct GitSnapshot {
    pub repository: Repository,
    pub status: WorkingDirectoryStatus,
    pub commits: Vec<Commit>,
    pub branches: Vec<Branch>,
}