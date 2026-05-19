//! Translation of multi-commit-operation.ts → all multi-commit-operation types.
//!
//! Rebase, cherry-pick, squash, reorder, and merge share a common "step
//! machine" described here.  The concrete operation details are captured in
//! `MultiCommitOperationDetail`.

use crate::models::branch::Branch;
use crate::models::commit::{Commit, CommitContext, CommitOneLine};

// ---------------------------------------------------------------------------
// MultiCommitOperationKind
// ---------------------------------------------------------------------------

/// Which kind of multi-commit operation is in progress.
///
/// Maps to `MultiCommitOperationKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiCommitOperationKind {
    Rebase,
    CherryPick,
    Squash,
    Merge,
    Reorder,
}

impl std::fmt::Display for MultiCommitOperationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rebase => write!(f, "Rebase"),
            Self::CherryPick => write!(f, "Cherry-pick"),
            Self::Squash => write!(f, "Squash"),
            Self::Merge => write!(f, "Merge"),
            Self::Reorder => write!(f, "Reorder"),
        }
    }
}

// ---------------------------------------------------------------------------
// Step kinds
// ---------------------------------------------------------------------------

/// Which phase of the UI flow the operation is currently in.
///
/// Maps to `MultiCommitOperationStepKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiCommitOperationStepKind {
    /// The user is picking which other branch is involved.
    ChooseBranch,
    /// Warning that the operation will require a force-push.
    WarnForcePush,
    /// Progress indicator is showing.
    ShowProgress,
    /// Conflict list is showing; the user must resolve conflicts.
    ShowConflicts,
    /// The user navigated away from the conflict dialog (banner shown instead).
    HideConflicts,
    /// The user tried to abort but has already resolved some conflicts.
    ConfirmAbort,
    /// A "create new branch" dialog appeared mid-operation (e.g. cherry-pick).
    CreateBranch,
}

// ---------------------------------------------------------------------------
// Per-step data
// ---------------------------------------------------------------------------
//
// In TypeScript these are separate interface types.  In Rust we model them as
// individual structs and bundle them into the `MultiCommitOperationStep` enum.

/// Data for the `ChooseBranch` step.
pub struct ChooseBranchStep {
    pub default_branch: Option<Branch>,
    pub current_branch: Branch,
    pub all_branches: Vec<Branch>,
    pub recent_branches: Vec<Branch>,
    pub initial_branch: Option<Branch>,
}

/// Data for the `WarnForcePush` step.
pub struct WarnForcePushStep {
    pub base_branch: Branch,
    pub target_branch: Branch,
    pub commits: Vec<CommitOneLine>,
}

/// Data for the `ShowConflicts` and `HideConflicts` / `ConfirmAbort` steps.
///
/// The TypeScript version references `MultiCommitOperationConflictState` from
/// `app-state`; we use a plain string here as a placeholder — replace with
/// your own conflict-state type when you port `app-state`.
pub struct ConflictStep {
    /// A description of the conflicted state (replace with a richer type later).
    pub conflict_description: String,
}

/// Data for the `CreateBranch` step (used during cherry-pick to a new branch).
pub struct CreateBranchStep {
    pub all_branches: Vec<Branch>,
    pub default_branch: Option<Branch>,
    pub upstream_default_branch: Option<Branch>,
    pub target_branch_name: String,
}

/// The complete set of possible step states for a multi-commit operation.
///
/// Maps to the TypeScript `MultiCommitOperationStep` union type.
pub enum MultiCommitOperationStep {
    ChooseBranch(ChooseBranchStep),
    WarnForcePush(WarnForcePushStep),
    ShowProgress,
    ShowConflicts(ConflictStep),
    HideConflicts(ConflictStep),
    ConfirmAbort(ConflictStep),
    CreateBranch(CreateBranchStep),
}

impl MultiCommitOperationStep {
    pub fn kind(&self) -> MultiCommitOperationStepKind {
        match self {
            Self::ChooseBranch(_) => MultiCommitOperationStepKind::ChooseBranch,
            Self::WarnForcePush(_) => MultiCommitOperationStepKind::WarnForcePush,
            Self::ShowProgress => MultiCommitOperationStepKind::ShowProgress,
            Self::ShowConflicts(_) => MultiCommitOperationStepKind::ShowConflicts,
            Self::HideConflicts(_) => MultiCommitOperationStepKind::HideConflicts,
            Self::ConfirmAbort(_) => MultiCommitOperationStepKind::ConfirmAbort,
            Self::CreateBranch(_) => MultiCommitOperationStepKind::CreateBranch,
        }
    }

    /// Returns `true` when the step involves an active or acknowledged conflict.
    pub fn is_conflict_step(&self) -> bool {
        matches!(self, Self::ShowConflicts(_) | Self::ConfirmAbort(_))
    }
}

// ---------------------------------------------------------------------------
// Per-operation detail payloads
// ---------------------------------------------------------------------------

/// Details specific to a **squash** operation.
pub struct SquashDetails {
    pub commits: Vec<Commit>,
    pub current_tip: String,
    pub last_retained_commit_ref: Option<String>,
    /// The commit all selected commits are squashed onto.
    pub target_commit: Commit,
    pub commit_context: CommitContext,
}

/// Details specific to a **reorder** operation.
pub struct ReorderDetails {
    pub commits: Vec<Commit>,
    pub current_tip: String,
    pub last_retained_commit_ref: Option<String>,
    /// The commit before which the reordered commits will be placed.
    pub before_commit: Option<Commit>,
}

/// Details specific to a **cherry-pick** operation.
pub struct CherryPickDetails {
    /// The branch the user started on (source of commits).
    pub source_branch: Option<Branch>,
    pub commits: Vec<CommitOneLine>,
    /// Whether a new branch was created during the operation.
    pub branch_created: bool,
}

/// Details specific to a **rebase** operation.
pub struct RebaseDetails {
    /// The branch supplying commits (chosen in `ChooseBranch`).
    pub source_branch: Option<Branch>,
    pub commits: Vec<CommitOneLine>,
    pub current_tip: String,
}

/// Details specific to a **merge** operation.
pub struct MergeDetails {
    pub source_branch: Option<Branch>,
    /// `true` when "squash and merge" was chosen instead of a regular merge.
    pub is_squash: bool,
}

/// The operation-specific payload for whichever operation is in flight.
///
/// Maps to the TypeScript `MultiCommitOperationDetail` union type.
pub enum MultiCommitOperationDetail {
    Squash(SquashDetails),
    Reorder(ReorderDetails),
    CherryPick(CherryPickDetails),
    Rebase(RebaseDetails),
    Merge(MergeDetails),
}

impl MultiCommitOperationDetail {
    pub fn kind(&self) -> MultiCommitOperationKind {
        match self {
            Self::Squash(_) => MultiCommitOperationKind::Squash,
            Self::Reorder(_) => MultiCommitOperationKind::Reorder,
            Self::CherryPick(_) => MultiCommitOperationKind::CherryPick,
            Self::Rebase(_) => MultiCommitOperationKind::Rebase,
            Self::Merge(_) => MultiCommitOperationKind::Merge,
        }
    }
}

// ---------------------------------------------------------------------------
// Progress  (progress.ts — embedded here; extract to progress.rs if desired)
// ---------------------------------------------------------------------------

/// Progress reported while a multi-commit operation is running.
///
/// Maps to `IMultiCommitOperationProgress`.
#[derive(Debug, Clone)]
pub struct MultiCommitOperationProgress {
    /// Overall progress as a fraction in `[0.0, 1.0]`.
    pub value: f64,
    pub title: Option<String>,
    pub description: Option<String>,
    /// Subject line of the commit currently being applied.
    pub current_commit_summary: String,
    /// 1-based index of the commit being applied.
    pub position: usize,
    /// Total number of commits in this operation.
    pub total_commit_count: usize,
}
