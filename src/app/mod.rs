pub mod message;
pub mod update;
pub mod view;

use std::path::PathBuf;

use crate::models::branch::Branch;
use crate::models::commit::Commit;
use crate::models::repository::Repository;
use crate::models::status::WorkingDirectoryStatus;

// Re-export so `main.rs` can still use `app::App`, `app::Message`, etc.
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
    /// The one-line subject typed into the Summary field.
    pub commit_summary: String,
    /// The optional multi-line body typed into the Description field.
    pub commit_description: String,
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
        }
    }
}
