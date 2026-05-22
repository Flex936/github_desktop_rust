pub mod message;
pub mod update;
pub mod view;

use std::path::PathBuf;

use crate::models::branch::Branch;
use crate::models::commit::Commit;
use crate::models::repository::Repository;
use crate::models::status::WorkingDirectoryStatus;

pub use message::{GitSnapshot, Message};
pub use update::update;
pub use view::view;

pub struct App {
    pub repo_path: Option<PathBuf>,
    pub repository: Option<Repository>,
    pub status: Option<WorkingDirectoryStatus>,
    pub commits: Vec<Commit>,
    pub branches: Vec<Branch>,
    pub selected_branch: Option<String>,
    pub error: Option<String>,

    // ── Commit panel state ────────────────────────────────────────────────
    pub commit_summary: String,
    pub commit_description: String,

    // ── Diff view state ───────────────────────────────────────────────────
    /// The file the user clicked on in the sidebar; switches the main panel
    /// from Commit History to the Diff view.
    pub selected_file_path: Option<PathBuf>,
    /// The rendered patch text for `selected_file_path`, populated
    /// asynchronously after `FileClicked`.
    pub current_diff: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            repo_path: None,
            repository: None,
            status: None,
            commits: Vec::new(),
            branches: Vec::new(),
            selected_branch: None,
            error: None,
            commit_summary: String::new(),
            commit_description: String::new(),
            selected_file_path: None,
            current_diff: None,
        }
    }
}