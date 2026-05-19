//! Low-level git operations powered entirely by libgit2 (`git2` crate).
//!
//! **No child processes, no regex, no porcelain-text parsing.**
//! Every function talks directly to libgit2's native C API through the
//! safe Rust bindings.
use chrono::{DateTime, FixedOffset, TimeZone};
use git2::{Repository as Git2Repository, Sort, Status, StatusOptions};
use std::path::{Path, PathBuf};

use crate::models::commit::{Commit, CommitIdentity};
use crate::models::repository::{Repository, WorkflowPreferences};
use crate::models::status::{
    AppFileStatus, AppFileStatusKind, ConflictedFileStatus, CopiedOrRenamedFileStatus,
    DiffSelection, FileChange, PlainFileStatus, SubmoduleStatus, UnmergedEntrySummary,
    UntrackedFileStatus, WorkingDirectoryFileChange, WorkingDirectoryStatus,
};

pub struct GitEngine;

impl GitEngine {
    // -----------------------------------------------------------------------
    // Repository discovery
    // -----------------------------------------------------------------------

    /// Opens a local directory and verifies it is a valid Git repository.
    ///
    /// Returns a fully-populated [`Repository`] model on success.
    pub fn open_repository(path: &Path, id: i64) -> Result<Repository, git2::Error> {
        // Validate the path via libgit2 before constructing our model.
        let _repo = Git2Repository::open(path)?;

        Ok(Repository {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path: path.to_path_buf(),
            id,
            github_repository: None, // Populated later when remotes are inspected.
            missing: false,
            alias: None,
            workflow_preferences: WorkflowPreferences::default(),
            is_tutorial_repository: false,
        })
    }

    // -----------------------------------------------------------------------
    // Working-directory status
    // -----------------------------------------------------------------------

    /// Queries the working directory and index for all pending changes and maps
    /// them into Desktop's [`WorkingDirectoryStatus`] model.
    ///
    /// # Implementation notes
    ///
    /// * Uses `repo.statuses()` with libgit2's native bitflag API —
    ///   **no child process, no regex, no porcelain-text parsing**.
    /// * Rename detection is enabled for both the index and the working tree via
    ///   `StatusOptions::renames_head_to_index` /
    ///   `StatusOptions::renames_index_to_workdir`.
    /// * Conflict-marker counting reads the raw file bytes with a sliding-window
    ///   byte search — no regex involved.
    /// * Submodule status is resolved via `repo.submodule_status()` when the
    ///   path matches a registered submodule; returns `None` otherwise.
    pub fn get_working_directory_status(
        repo: &Git2Repository,
    ) -> Result<WorkingDirectoryStatus, git2::Error> {
        let mut opts = StatusOptions::new();
        opts
            // Surface untracked files and recurse into untracked directories.
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            // Skip ignored paths — Desktop never shows them.
            .include_ignored(false)
            // Skip completely clean files.
            .include_unmodified(false)
            // Let libgit2 detect renames in both the index and the working tree.
            .renames_head_to_index(true)
            .renames_index_to_workdir(true);

        let statuses = repo.statuses(Some(&mut opts))?;
        let workdir = repo.workdir(); // `None` for bare repositories.

        let mut files: Vec<WorkingDirectoryFileChange> = Vec::with_capacity(statuses.len());

        for entry in statuses.iter() {
            let flags = entry.status();

            // Skip entries that are clean or ignored (shouldn't appear given
            // the options above, but guard defensively).
            if flags.is_empty()
                || flags.contains(Status::CURRENT)
                || flags.contains(Status::IGNORED)
            {
                continue;
            }

            // Git paths are always valid UTF-8 by convention.
            let Ok(raw_path) = entry.path() else {
                continue;
            };
            let path = PathBuf::from(raw_path);

            // Probe the submodule registry; returns `None` for ordinary files.
            let submodule_status = submodule_status_for(repo, &path);

            let app_status =
                build_app_file_status(repo, &entry, flags, &path, workdir, submodule_status);

            files.push(WorkingDirectoryFileChange {
                file_change: FileChange {
                    path,
                    status: app_status,
                },
                // New entries start fully selected — ready to commit as-is.
                selection: DiffSelection::select_all(),
            });
        }

        Ok(WorkingDirectoryStatus::from_files(files))
    }

    pub fn get_commit_history(
        repo: &Git2Repository,
        limit: usize,
    ) -> Result<Vec<Commit>, git2::Error> {
        // ── 1. Set up the rev-walker ─────────────────────────────────────
        let mut walk = repo.revwalk()?;

        // Start from HEAD; returns Err when the repository is empty / unborn.
        walk.push_head()?;

        // Reverse-chronological, topological sort — mirrors `git log` output.
        walk.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)?;

        // ── 2. Collect up to `limit` commits ─────────────────────────────
        let mut commits: Vec<Commit> = Vec::with_capacity(limit.min(256));

        for oid_result in walk.take(limit) {
            let oid = oid_result?;
            let git2_commit = repo.find_commit(oid)?;

            // ── SHA strings ─────────────────────────────────────────────
            let sha = oid.to_string(); // full 40-char hex
            let short_sha = sha[..sha.len().min(9)].to_string();

            // ── Message: split summary / body on first blank line ───────
            let raw_message = git2_commit.message().unwrap_or("").trim_end().to_string();
            let (summary, body) = split_message(&raw_message);

            // ── Author ──────────────────────────────────────────────────
            let git2_author = git2_commit.author();
            let author = CommitIdentity::new(
                git2_author.name().unwrap_or(""),
                git2_author.email().unwrap_or(""),
                git2_time_to_datetime(git2_author.when()),
            );

            // ── Committer ───────────────────────────────────────────────
            let git2_committer = git2_commit.committer();
            let committer = CommitIdentity::new(
                git2_committer.name().unwrap_or(""),
                git2_committer.email().unwrap_or(""),
                git2_time_to_datetime(git2_committer.when()),
            );

            // ── Parent SHAs (all of them — needed for merge detection) ──
            let parent_shas: Vec<String> =
                git2_commit.parent_ids().map(|id| id.to_string()).collect();

            commits.push(Commit {
                sha,
                short_sha,
                summary,
                body,
                author,
                committer,
                parent_shas,
                // Tag and co-author resolution requires a separate pass;
                // leave empty here exactly as the TS getCommits flow does.
                tags: Vec::new(),
                co_authors: Vec::new(),
            });
        }

        Ok(commits)
    }
}

// ---------------------------------------------------------------------------
// Status-mapping helpers (private)
// ---------------------------------------------------------------------------

/// Converts a single `StatusEntry` and its bitflags into an [`AppFileStatus`].
///
/// Priority order matches GitHub Desktop's TypeScript logic:
///   1. Conflicted (requires manual resolution before anything else)
///   2. Untracked  (WT_NEW — not staged at all)
///   3. Renamed / Copied (detected via diff deltas)
///   4. Plain: New → Deleted → Modified (index flags before working-tree flags)
fn build_app_file_status(
    repo: &Git2Repository,
    entry: &git2::StatusEntry<'_>,
    flags: Status,
    path: &Path,
    workdir: Option<&Path>,
    submodule_status: Option<SubmoduleStatus>,
) -> AppFileStatus {
    // ── 1. Conflicted ───────────────────────────────────────────────────────
    if flags.contains(Status::CONFLICTED) {
        // Determine conflict shape by inspecting index stages 1/2/3.
        let action = conflict_action_for(repo, path);

        // Count `<<<<<<<` markers in the working-tree copy of the file.
        // Binary / mode-change conflicts will fail to open cleanly, giving
        // `None`, which correctly signals "manual conflict" to the UI.
        let conflict_marker_count = workdir
            .map(|wd| wd.join(path))
            .as_deref()
            .and_then(|abs| count_conflict_markers(abs).ok());

        return AppFileStatus::Conflicted(ConflictedFileStatus {
            action,
            conflict_marker_count,
            submodule_status,
        });
    }

    // ── 2. Untracked ────────────────────────────────────────────────────────
    // WT_NEW means "exists on disk, not in the index at all".
    if flags.contains(Status::WT_NEW) {
        return AppFileStatus::Untracked(UntrackedFileStatus { submodule_status });
    }

    // ── 3. Renamed / Copied ─────────────────────────────────────────────────
    // libgit2 stores rename info in the DiffDelta attached to the entry.
    // We check both the staged (head_to_index) and unstaged (index_to_workdir)
    // deltas and take the first valid old-path we find.
    if flags.intersects(Status::INDEX_RENAMED | Status::WT_RENAMED) {
        if let Some(old_path) = old_path_for(entry) {
            return AppFileStatus::CopiedOrRenamed(CopiedOrRenamedFileStatus {
                kind: AppFileStatusKind::Renamed,
                old_path,
                submodule_status,
            });
        }
        // Rename flag set but no recoverable old path — fall through.
    }

    // ── 4. Plain: New / Deleted / Modified ──────────────────────────────────
    //
    // Index flags (staged intent) are checked before working-tree flags so
    // that staged changes surface first, matching Desktop's UX behaviour.
    let kind = if flags.contains(Status::INDEX_NEW) {
        AppFileStatusKind::New
    } else if flags.intersects(Status::INDEX_DELETED | Status::WT_DELETED) {
        AppFileStatusKind::Deleted
    } else {
        // Covers INDEX_MODIFIED | WT_MODIFIED | INDEX_TYPECHANGE | WT_TYPECHANGE.
        AppFileStatusKind::Modified
    };

    AppFileStatus::Plain(PlainFileStatus {
        kind,
        submodule_status,
    })
}

// ---------------------------------------------------------------------------
// Rename / old-path recovery
// ---------------------------------------------------------------------------

/// Recovers the pre-rename path from the diff deltas attached to a status entry.
///
/// libgit2 stores rename information in the `DiffDelta` embedded in each
/// `StatusEntry`.  We check the staged delta (`head_to_index`) first and
/// fall back to the unstaged delta (`index_to_workdir`).
fn old_path_for(entry: &git2::StatusEntry<'_>) -> Option<PathBuf> {
    // Staged rename: HEAD → index delta.
    if let Some(delta) = entry.head_to_index() {
        let old = delta.old_file().path()?;
        let new = delta.new_file().path()?;
        if old != new {
            return Some(old.to_path_buf());
        }
    }
    // Unstaged rename: index → workdir delta.
    if let Some(delta) = entry.index_to_workdir() {
        let old = delta.old_file().path()?;
        let new = delta.new_file().path()?;
        if old != new {
            return Some(old.to_path_buf());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Conflict-type detection
// ---------------------------------------------------------------------------

/// Determines which kind of merge conflict affects a path by inspecting the
/// three index stages that git uses during a conflict.
///
/// | stages present | porcelain | meaning             |
/// |---------------|-----------|---------------------|
/// | 1 only        | DD        | Both deleted        |
/// | 2 only        | AU        | Added by us         |
/// | 1 + 2         | UD        | Deleted by them     |
/// | 3 only        | UA        | Added by them       |
/// | 1 + 3         | DU        | Deleted by us       |
/// | 2 + 3         | AA        | Both added          |
/// | 1 + 2 + 3     | UU        | Both modified       |
///
/// Stage meanings: **1** = common ancestor, **2** = ours, **3** = theirs.
fn conflict_action_for(repo: &Git2Repository, path: &Path) -> UnmergedEntrySummary {
    // Read the current index directly — this is a cheap in-memory operation.
    let Ok(index) = repo.index() else {
        return UnmergedEntrySummary::BothModified;
    };

    let stage1 = index.get_path(path, 1).is_some();
    let stage2 = index.get_path(path, 2).is_some();
    let stage3 = index.get_path(path, 3).is_some();

    match (stage1, stage2, stage3) {
        (true, false, false) => UnmergedEntrySummary::BothDeleted, // DD
        (false, true, false) => UnmergedEntrySummary::AddedByUs,   // AU
        (true, true, false) => UnmergedEntrySummary::DeletedByThem, // UD
        (false, false, true) => UnmergedEntrySummary::AddedByThem, // UA
        (true, false, true) => UnmergedEntrySummary::DeletedByUs,  // DU
        (false, true, true) => UnmergedEntrySummary::BothAdded,    // AA
        _ => UnmergedEntrySummary::BothModified,                   // UU (or unknown)
    }
}

// ---------------------------------------------------------------------------
// Conflict-marker counting
// ---------------------------------------------------------------------------

/// Counts the number of unresolved conflict blocks in a file by scanning for
/// `<<<<<<<` byte sequences.
///
/// Each `<<<<<<<` begins one conflict block, so the count equals the number
/// of conflicts still outstanding.  We read raw bytes and use a sliding-window
/// comparison — **no regex, no child process, no UTF-8 assumption**.
///
/// Returns `Err` when the file cannot be read (e.g. binary conflict, deleted
/// file), which the caller treats as a manual / marker-less conflict.
fn count_conflict_markers(path: &Path) -> std::io::Result<u32> {
    const MARKER: &[u8] = b"<<<<<<<";

    let bytes = std::fs::read(path)?;
    let count = bytes
        .windows(MARKER.len())
        .filter(|window| *window == MARKER)
        .count();

    Ok(count as u32)
}

// ---------------------------------------------------------------------------
// Submodule status
// ---------------------------------------------------------------------------

/// Attempts to query libgit2's submodule registry for `path`.
///
/// Returns `None` when the path is not a registered submodule or the query
/// fails (ordinary files always fall into this case).
fn submodule_status_for(repo: &Git2Repository, path: &Path) -> Option<SubmoduleStatus> {
    let path_str = path.to_str()?;

    // `submodule_status` errors immediately for non-submodule paths.
    let sm_flags = repo
        .submodule_status(path_str, git2::SubmoduleIgnore::None)
        .ok()?;

    Some(SubmoduleStatus {
        // The commit recorded in the index / HEAD differs from the submodule's
        // current HEAD commit.
        commit_changed: sm_flags
            .intersects(git2::SubmoduleStatus::INDEX_MODIFIED | git2::SubmoduleStatus::WD_MODIFIED),
        // There are staged or unstaged edits *inside* the submodule's working tree.
        modified_changes: sm_flags.intersects(
            git2::SubmoduleStatus::WD_INDEX_MODIFIED | git2::SubmoduleStatus::WD_WD_MODIFIED,
        ),
        // Untracked files exist inside the submodule.
        untracked_changes: sm_flags.contains(git2::SubmoduleStatus::WD_UNTRACKED),
    })
}

// ---------------------------------------------------------------------------
// Commit-history helpers (private)
// ---------------------------------------------------------------------------

/// Converts a libgit2 `Time` (Unix seconds + signed tz offset in minutes)
/// into a `chrono::DateTime<FixedOffset>`.
///
/// Uses the `_opt` / `single_opt` constructors throughout so the function
/// is panic-free even with malformed repository data.
///
/// Falls back to UTC at the Unix epoch when either conversion is
/// out-of-range (practically impossible with real git data).
fn git2_time_to_datetime(time: git2::Time) -> DateTime<FixedOffset> {
    // git2 gives us the offset in whole minutes; chrono wants whole seconds.
    let offset_secs = time.offset_minutes() * 60;

    let zone = FixedOffset::east_opt(offset_secs)
        .unwrap_or_else(|| FixedOffset::east_opt(0).expect("UTC is always valid"));

    zone.timestamp_opt(time.seconds(), 0)
        .single()
        .unwrap_or_else(|| {
            FixedOffset::east_opt(0)
                .expect("UTC is always valid")
                .timestamp_opt(0, 0)
                .single()
                .expect("Unix epoch is always valid")
        })
}

/// Splits a raw git commit message into `(summary, body)`.
///
/// Git's convention is: the subject line, then a blank line, then the body.
/// We replicate that split without any regex.
///
/// Returns an empty `body` string when no blank line is present.
fn split_message(raw: &str) -> (String, String) {
    // Find the first completely blank line (only whitespace / empty).
    let mut lines = raw.splitn(2, "\n\n");
    let summary = lines.next().unwrap_or("").trim().to_string();
    let body = lines.next().unwrap_or("").trim_end().to_string();
    (summary, body)
}
