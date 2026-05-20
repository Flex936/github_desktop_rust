use crate::git_engine::GitEngine;
use crate::models::status::{
    AppFileStatus, AppFileStatusKind, ConflictedFileStatus, CopiedOrRenamedFileStatus,
    DiffSelection, FileChange, PlainFileStatus, SubmoduleStatus, UnmergedEntrySummary,
    UntrackedFileStatus, WorkingDirectoryFileChange, WorkingDirectoryStatus,
};
use git2::{Repository as Git2Repository, Status, StatusOptions};
use std::path::{Path, PathBuf};

impl GitEngine {
    pub fn get_working_directory_status(
        repo: &Git2Repository,
    ) -> Result<WorkingDirectoryStatus, git2::Error> {
        let mut opts = StatusOptions::new();
        // Explicitly force git to look at the working directory
        opts.show(git2::StatusShow::IndexAndWorkdir)
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false)
            .include_unmodified(false)
            .renames_head_to_index(true)
            .renames_index_to_workdir(true);

        let statuses = repo.statuses(Some(&mut opts))?;
        let workdir = repo.workdir();

        let mut files: Vec<WorkingDirectoryFileChange> = Vec::with_capacity(statuses.len());

        for entry in statuses.iter() {
            let flags = entry.status();

            // FIX: We removed `Status::CURRENT` because it equals 0 and causes
            // `contains` to always return true, skipping all files!
            // `is_empty()` safely checks if the file is unmodified.
            if flags.is_empty() || flags.contains(Status::IGNORED) {
                continue;
            }

            let path_bytes = entry.path_bytes();
            let path_str = match std::str::from_utf8(path_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let path = PathBuf::from(path_str);

            let submodule_status = submodule_status_for(repo, &path);
            let app_status =
                build_app_file_status(repo, &entry, flags, &path, workdir, submodule_status);

            files.push(WorkingDirectoryFileChange {
                file_change: FileChange {
                    path,
                    status: app_status,
                },
                selection: DiffSelection::select_all(),
            });
        }

        Ok(WorkingDirectoryStatus::from_files(files))
    }
}

fn build_app_file_status(
    repo: &Git2Repository,
    entry: &git2::StatusEntry<'_>,
    flags: Status,
    path: &Path,
    workdir: Option<&Path>,
    submodule_status: Option<SubmoduleStatus>,
) -> AppFileStatus {
    if flags.contains(Status::CONFLICTED) {
        let action = conflict_action_for(repo, path);
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

    if flags.contains(Status::WT_NEW) {
        return AppFileStatus::Untracked(UntrackedFileStatus { submodule_status });
    }

    if flags.intersects(Status::INDEX_RENAMED | Status::WT_RENAMED) {
        if let Some(old_path) = old_path_for(entry) {
            return AppFileStatus::CopiedOrRenamed(CopiedOrRenamedFileStatus {
                kind: AppFileStatusKind::Renamed,
                old_path,
                submodule_status,
            });
        }
    }

    let kind = if flags.contains(Status::INDEX_NEW) {
        AppFileStatusKind::New
    } else if flags.intersects(Status::INDEX_DELETED | Status::WT_DELETED) {
        AppFileStatusKind::Deleted
    } else {
        AppFileStatusKind::Modified
    };

    AppFileStatus::Plain(PlainFileStatus {
        kind,
        submodule_status,
    })
}

fn old_path_for(entry: &git2::StatusEntry<'_>) -> Option<PathBuf> {
    if let Some(delta) = entry.head_to_index() {
        let old = delta.old_file().path()?;
        let new = delta.new_file().path()?;
        if old != new {
            return Some(old.to_path_buf());
        }
    }
    if let Some(delta) = entry.index_to_workdir() {
        let old = delta.old_file().path()?;
        let new = delta.new_file().path()?;
        if old != new {
            return Some(old.to_path_buf());
        }
    }
    None
}

fn conflict_action_for(repo: &Git2Repository, path: &Path) -> UnmergedEntrySummary {
    let Ok(index) = repo.index() else {
        return UnmergedEntrySummary::BothModified;
    };
    let stage1 = index.get_path(path, 1).is_some();
    let stage2 = index.get_path(path, 2).is_some();
    let stage3 = index.get_path(path, 3).is_some();

    match (stage1, stage2, stage3) {
        (true, false, false) => UnmergedEntrySummary::BothDeleted,
        (false, true, false) => UnmergedEntrySummary::AddedByUs,
        (true, true, false) => UnmergedEntrySummary::DeletedByThem,
        (false, false, true) => UnmergedEntrySummary::AddedByThem,
        (true, false, true) => UnmergedEntrySummary::DeletedByUs,
        (false, true, true) => UnmergedEntrySummary::BothAdded,
        _ => UnmergedEntrySummary::BothModified,
    }
}

fn count_conflict_markers(path: &Path) -> std::io::Result<u32> {
    const MARKER: &[u8] = b"<<<<<<<";
    let bytes = std::fs::read(path)?;
    let count = bytes
        .windows(MARKER.len())
        .filter(|window| *window == MARKER)
        .count();
    Ok(count as u32)
}

fn submodule_status_for(repo: &Git2Repository, path: &Path) -> Option<SubmoduleStatus> {
    let path_str = path.to_str()?;
    let sm_flags = repo
        .submodule_status(path_str, git2::SubmoduleIgnore::None)
        .ok()?;

    Some(SubmoduleStatus {
        commit_changed: sm_flags
            .intersects(git2::SubmoduleStatus::INDEX_MODIFIED | git2::SubmoduleStatus::WD_MODIFIED),
        modified_changes: sm_flags.intersects(
            git2::SubmoduleStatus::WD_INDEX_MODIFIED | git2::SubmoduleStatus::WD_WD_MODIFIED,
        ),
        untracked_changes: sm_flags.contains(git2::SubmoduleStatus::WD_UNTRACKED),
    })
}
