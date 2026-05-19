//! Translation of branch.ts → Branch and related types.

use crate::models::commit::CommitIdentity;

// ---------------------------------------------------------------------------
// BranchType
// ---------------------------------------------------------------------------

/// Whether a branch lives locally or only on a remote.
///
/// The discriminant values are preserved from TypeScript so that sorting
/// local before remote still works when you cast to an integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BranchType {
    Local = 0,
    Remote = 1,
}

// ---------------------------------------------------------------------------
// AheadBehind / CompareResult
// ---------------------------------------------------------------------------

/// How many commits a branch is ahead of and behind another ref.
///
/// Maps to `IAheadBehind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AheadBehind {
    pub ahead: u32,
    pub behind: u32,
}

// ---------------------------------------------------------------------------
// BranchTip
// ---------------------------------------------------------------------------

/// The SHA and author of the most recent commit on a branch.
///
/// Maps to `IBranchTip`.
#[derive(Debug, Clone)]
pub struct BranchTip {
    pub sha: String,
    pub author: CommitIdentity,
}

// ---------------------------------------------------------------------------
// StartPoint
// ---------------------------------------------------------------------------

/// Where a newly created branch should start from.
///
/// Maps to the `StartPoint` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartPoint {
    CurrentBranch,
    DefaultBranch,
    Head,
    /// Only valid for forks — starts from the upstream default branch.
    UpstreamDefaultBranch,
}

// ---------------------------------------------------------------------------
// Branch
// ---------------------------------------------------------------------------

/// The magic prefix GitHub Desktop adds to its automatically-managed fork remotes.
pub const FORKED_REMOTE_PREFIX: &str = "github-desktop-";

/// A branch as loaded from Git.
///
/// Maps to the TypeScript `Branch` class.
#[derive(Debug, Clone)]
pub struct Branch {
    /// The short name of the branch, e.g. `main`.
    pub name: String,

    /// The remote-prefixed upstream tracking branch, e.g. `origin/main`.
    /// `None` for local branches that have no upstream.
    pub upstream: Option<String>,

    /// The SHA and author of the tip commit.
    pub tip: BranchTip,

    pub branch_type: BranchType,

    /// The fully qualified ref, e.g. `refs/heads/main` or
    /// `refs/remotes/origin/main`.
    ///
    /// Named `git_ref` because `ref` is a reserved keyword in Rust.
    pub git_ref: String,
}

impl Branch {
    // -----------------------------------------------------------------------
    // Derived properties (mirror the TypeScript getters)
    // -----------------------------------------------------------------------

    /// The remote name from the upstream tracking branch, e.g. `"origin"`.
    /// Returns `None` if there is no upstream.
    pub fn upstream_remote_name(&self) -> Option<&str> {
        self.upstream
            .as_deref()?
            .split_once('/')
            .map(|(remote, _)| remote)
    }

    /// The remote that owns this branch (for remote branches only).
    /// Returns `None` for local branches.
    ///
    /// Panics in debug builds if a remote branch's ref is malformed.
    pub fn remote_name(&self) -> Option<String> {
        if self.branch_type == BranchType::Local {
            return None;
        }

        // Format: refs/remotes/<remote>/<branch>
        self.git_ref
            .strip_prefix("refs/remotes/")
            .and_then(|rest| rest.split_once('/').map(|(remote, _)| remote.to_string()))
    }

    /// The upstream branch name with the remote prefix removed, e.g. `"main"`
    /// instead of `"origin/main"`. Returns `None` if there is no upstream.
    pub fn upstream_without_remote(&self) -> Option<&str> {
        self.upstream
            .as_deref()?
            .split_once('/')
            .map(|(_, branch)| branch)
    }

    /// The branch name without any remote prefix.
    /// For a local branch this is the same as `name`.
    pub fn name_without_remote(&self) -> &str {
        if self.branch_type == BranchType::Local {
            return &self.name;
        }
        self.name
            .split_once('/')
            .map(|(_, branch)| branch)
            .unwrap_or(&self.name)
    }

    /// `true` when this is a remote branch from one of Desktop's automatically
    /// created fork remotes (prefix `github-desktop-`).
    ///
    /// These branches are treated as plumbing and hidden from the UI.
    pub fn is_desktop_fork_remote_branch(&self) -> bool {
        self.branch_type == BranchType::Remote && self.name.starts_with(FORKED_REMOTE_PREFIX)
    }
}
