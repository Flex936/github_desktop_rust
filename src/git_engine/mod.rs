pub mod branch;
pub mod checkout;
pub mod history;
pub mod status;

use crate::models::repository::{Repository, WorkflowPreferences};
use git2::Repository as Git2Repository;
use std::path::Path;

pub struct GitEngine;

impl GitEngine {
    /// Opens a local directory and verifies it is a valid Git repository.
    pub fn open_repository(path: &Path, id: i64) -> Result<Repository, git2::Error> {
        let _repo = Git2Repository::open(path)?;

        Ok(Repository {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path: path.to_path_buf(),
            id,
            github_repository: None,
            missing: false,
            alias: None,
            workflow_preferences: WorkflowPreferences::default(),
            is_tutorial_repository: false,
        })
    }
}
