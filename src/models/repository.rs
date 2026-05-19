//! Translations of:
//!   owner.ts             →  Owner
//!   github-repository.ts →  GitHubRepository, GitHubRepositoryPermission
//!   repository.ts        →  Repository and helpers
//!   workflow-preferences.ts → ForkContributionTarget, WorkflowPreferences

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ForkContributionTarget / WorkflowPreferences  (workflow-preferences.ts)
// ---------------------------------------------------------------------------

/// Which copy of a forked repository the user wants to contribute to.
///
/// Maps to `ForkContributionTarget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForkContributionTarget {
    /// Contribute to the upstream (parent) repository — the default.
    #[default]
    Parent,
    /// Contribute to the fork itself.
    Self_,
}

/// Per-repository workflow settings chosen by the user.
///
/// Maps to `WorkflowPreferences`.
#[derive(Debug, Clone, Default)]
pub struct WorkflowPreferences {
    pub fork_contribution_target: Option<ForkContributionTarget>,
}

// ---------------------------------------------------------------------------
// Owner  (owner.ts)
// ---------------------------------------------------------------------------

/// The owner of a `GitHubRepository`.
///
/// Maps to the TypeScript `Owner` class.
#[derive(Debug, Clone, PartialEq)]
pub struct Owner {
    pub login: String,
    /// API endpoint, e.g. `"https://api.github.com"` or a GHE URL.
    pub endpoint: String,
    /// Database ID in the local app store (`id` in TypeScript).
    pub db_id: i64,
}

// ---------------------------------------------------------------------------
// GitHubRepository  (github-repository.ts)
// ---------------------------------------------------------------------------

/// The level of access the authenticated user has on a GitHub repository.
///
/// Maps to `GitHubRepositoryPermission`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitHubRepositoryPermission {
    Read,
    Write,
    Admin,
}

/// A GitHub (or GitHub Enterprise) repository.
///
/// Maps to the TypeScript `GitHubRepository` class.
#[derive(Debug, Clone)]
pub struct GitHubRepository {
    pub name: String,
    pub owner: Owner,
    /// Local database ID — unrelated to the GitHub API ID.
    pub db_id: i64,
    pub is_private: Option<bool>,
    pub html_url: Option<String>,
    pub clone_url: Option<String>,
    pub issues_enabled: Option<bool>,
    pub is_archived: Option<bool>,
    /// `None` means the permissions are unknown (treat as writable).
    pub permissions: Option<GitHubRepositoryPermission>,
    /// Present when this repository is a fork.
    ///
    /// `Box<T>` is required because the type is recursive.
    pub parent: Option<Box<GitHubRepository>>,
}

impl GitHubRepository {
    /// The API endpoint inherited from the owner.
    pub fn endpoint(&self) -> &str {
        &self.owner.endpoint
    }

    /// `"owner/name"` combined identifier.
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner.login, self.name)
    }

    /// `true` when this repository is a fork (i.e. has a parent).
    pub fn is_fork(&self) -> bool {
        self.parent.is_some()
    }
}

/// Returns `true` when the user can push to this repository.
///
/// When permissions are unknown we optimistically assume write access,
/// matching the behaviour of the TypeScript `hasWritePermission` function.
pub fn has_write_permission(repo: &GitHubRepository) -> bool {
    repo.permissions
        .map(|p| p != GitHubRepositoryPermission::Read)
        .unwrap_or(true)
}

// ---------------------------------------------------------------------------
// Repository  (repository.ts)
// ---------------------------------------------------------------------------

/// A local Git repository tracked by GitHub Desktop.
///
/// Maps to the TypeScript `Repository` class.
#[derive(Debug, Clone)]
pub struct Repository {
    pub name: String,
    /// Path to the working directory on disk.
    pub path: PathBuf,
    /// Local database ID.
    pub id: i64,
    /// Present when the repository is linked to a GitHub remote.
    pub github_repository: Option<GitHubRepository>,
    /// `true` when the working directory could not be found on the last check.
    pub missing: bool,
    /// A user-supplied display name that overrides the directory name.
    pub alias: Option<String>,
    pub workflow_preferences: WorkflowPreferences,
    /// `true` for repositories created as part of the onboarding tutorial.
    pub is_tutorial_repository: bool,
}

impl Repository {
    /// The display name: `owner/name` for GitHub repos, folder name otherwise.
    ///
    /// Matches the TypeScript `nameOf` free function.
    pub fn name_of(&self) -> String {
        match &self.github_repository {
            Some(gh) => gh.full_name(),
            None => self
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| self.path.display().to_string()),
        }
    }

    /// `true` when the repository is associated with a GitHub remote.
    pub fn has_github_repository(&self) -> bool {
        self.github_repository.is_some()
    }

    /// `true` when the repository is a GitHub fork.
    pub fn is_forked(&self) -> bool {
        self.github_repository
            .as_ref()
            .map(|gh| gh.is_fork())
            .unwrap_or(false)
    }

    /// Returns the effective fork-contribution target for this repository,
    /// defaulting to `Parent` when none has been configured.
    ///
    /// Matches `getForkContributionTarget` in TypeScript.
    pub fn fork_contribution_target(&self) -> ForkContributionTarget {
        self.workflow_preferences
            .fork_contribution_target
            .unwrap_or_default()
    }

    /// Returns the GitHub repository this repo *contributes to*:
    /// the fork itself when `ForkContributionTarget::Self_`, the upstream
    /// parent otherwise.
    ///
    /// Matches `getNonForkGitHubRepository` in TypeScript.
    pub fn contribution_target_repository(&self) -> Option<&GitHubRepository> {
        let gh = self.github_repository.as_ref()?;
        match self.fork_contribution_target() {
            ForkContributionTarget::Self_ => Some(gh),
            ForkContributionTarget::Parent => gh.parent.as_deref().or(Some(gh)),
        }
    }

    /// `true` when the user is contributing to the upstream parent repository.
    pub fn is_contributing_to_parent(&self) -> bool {
        self.is_forked() && self.fork_contribution_target() == ForkContributionTarget::Parent
    }
}

/// A snapshot of the local ahead/behind count and changed-file count for a
/// repository, used for sidebar badges.
///
/// Maps to `ILocalRepositoryState`.
#[derive(Debug, Clone)]
pub struct LocalRepositoryState {
    /// Ahead/behind relative to the tracking branch, or `None` if there is
    /// no tracking branch.
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    /// Number of files with uncommitted changes.
    pub changed_files_count: usize,
}
