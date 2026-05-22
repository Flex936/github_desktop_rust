use std::path::PathBuf;

use iced::Task;

use crate::app::{App, GitSnapshot, Message};
use crate::git_engine::GitEngine;
use crate::models::commit::CommitContext;
use crate::models::status::{WorkingDirectoryFileChange, WorkingDirectoryStatus};

const HISTORY_LIMIT: usize = 100;

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ── Repository lifecycle ──────────────────────────────────────────
        Message::LoadRepository(path) => {
            app.repo_path = Some(path.clone());
            app.repository = None;
            app.status = None;
            app.commits.clear();
            app.branches.clear();
            app.selected_branch = None;
            app.error = None;
            // Reset diff view whenever we load a new repository.
            app.selected_file_path = None;
            app.current_diff = None;
            Task::perform(fetch_git_snapshot(path), Message::DataLoaded)
        }
        Message::Refresh => match app.repo_path.clone() {
            Some(path) => {
                app.repository = None;
                app.status = None;
                app.commits.clear();
                app.branches.clear();
                app.error = None;
                // Reset diff view on every explicit refresh so stale diffs
                // don't linger after external changes.
                app.selected_file_path = None;
                app.current_diff = None;
                Task::perform(fetch_git_snapshot(path), Message::DataLoaded)
            }
            None => Task::none(),
        },
        Message::DataLoaded(Ok(snapshot)) => {
            let prev_selection = app.selected_branch.take();
            let new_names: Vec<String> =
                snapshot.branches.iter().map(|b| b.name.clone()).collect();

            app.selected_branch = prev_selection
                .filter(|prev| new_names.contains(prev))
                .or_else(|| new_names.first().cloned());

            app.repository = Some(snapshot.repository);
            app.status = Some(snapshot.status);
            app.commits = snapshot.commits;
            app.branches = snapshot.branches;
            app.error = None;
            Task::none()
        }
        Message::DataLoaded(Err(e)) => {
            app.error = Some(e);
            Task::none()
        }

        // ── Working-directory file selection ─────────────────────────────
        Message::ToggleAllFiles(is_checked) => {
            if let Some(status) = app.status.take() {
                app.status = Some(status.with_include_all_files(is_checked));
            }
            Task::none()
        }
        Message::ToggleFile(id, is_checked) => {
            if let Some(status) = app.status.take() {
                let new_files = status
                    .files
                    .into_iter()
                    .map(|f| {
                        if f.id() == id {
                            f.with_include_all(is_checked)
                        } else {
                            f
                        }
                    })
                    .collect();
                app.status = Some(WorkingDirectoryStatus::from_files(new_files));
            }
            Task::none()
        }

        // ── Branch operations ─────────────────────────────────────────────
        Message::BranchSelected(name) => {
            if app.selected_branch.as_deref() == Some(&name) {
                return Task::none();
            }
            app.selected_branch = Some(name.clone());
            app.error = None;

            let Some(path) = app.repo_path.clone() else {
                return Task::none();
            };

            Task::perform(perform_checkout(path, name), Message::CheckoutCompleted)
        }
        Message::CheckoutCompleted(Ok(branch_name)) => {
            app.selected_branch = Some(branch_name);
            app.error = None;

            let Some(path) = app.repo_path.clone() else {
                return Task::none();
            };
            Task::perform(fetch_git_snapshot(path), Message::DataLoaded)
        }
        Message::CheckoutCompleted(Err(e)) => {
            app.error = Some(format!("Checkout failed: {e}"));
            Task::none()
        }

        // ── Commit panel ──────────────────────────────────────────────────
        Message::CommitSummaryChanged(s) => {
            app.commit_summary = s;
            Task::none()
        }
        Message::CommitDescriptionChanged(s) => {
            app.commit_description = s;
            Task::none()
        }
        Message::Commit => {
            if app.commit_summary.trim().is_empty() {
                return Task::none();
            }

            let Some(path) = app.repo_path.clone() else {
                return Task::none();
            };

            let summary = app.commit_summary.clone();
            let description = app.commit_description.clone();
            let files: Vec<WorkingDirectoryFileChange> = app
                .status
                .as_ref()
                .map(|s| s.files.clone())
                .unwrap_or_default();

            Task::perform(
                perform_commit(path, summary, description, files),
                Message::CommitCompleted,
            )
        }
        Message::CommitCompleted(Ok(_sha)) => {
            app.commit_summary.clear();
            app.commit_description.clear();
            app.error = None;
            // After a commit the working tree is clean; dismiss any open diff.
            app.selected_file_path = None;
            app.current_diff = None;

            let Some(path) = app.repo_path.clone() else {
                return Task::none();
            };
            Task::perform(fetch_git_snapshot(path), Message::DataLoaded)
        }
        Message::CommitCompleted(Err(e)) => {
            app.error = Some(format!("Commit failed: {e}"));
            Task::none()
        }

        // ── Diff view ─────────────────────────────────────────────────────
        Message::FileClicked(path) => {
            // Show the diff panel immediately with a loading indicator,
            // then fire off the async diff computation.
            app.selected_file_path = Some(path.clone());
            app.current_diff = None; // cleared → UI shows "Loading diff…"
            app.error = None;

            let Some(repo_path) = app.repo_path.clone() else {
                return Task::none();
            };

            Task::perform(perform_diff(repo_path, path), Message::DiffLoaded)
        }
        Message::DiffLoaded(Ok(diff_text)) => {
            app.current_diff = Some(diff_text);
            Task::none()
        }
        Message::DiffLoaded(Err(e)) => {
            app.error = Some(format!("Diff failed: {e}"));
            Task::none()
        }
    }
}

// ── Async helpers ─────────────────────────────────────────────────────────────

async fn fetch_git_snapshot(path: PathBuf) -> Result<GitSnapshot, String> {
    let repository =
        GitEngine::open_repository(&path, 1).map_err(|e| format!("open: {e}"))?;
    let native = git2::Repository::open(&path).map_err(|e| format!("git2 open: {e}"))?;
    let status = GitEngine::get_working_directory_status(&native)
        .map_err(|e| format!("status: {e}"))?;
    let commits = GitEngine::get_commit_history(&native, HISTORY_LIMIT)
        .map_err(|e| format!("history: {e}"))?;
    let branches =
        GitEngine::get_branches(&native).map_err(|e| format!("branches: {e}"))?;

    Ok(GitSnapshot {
        repository,
        status,
        commits,
        branches,
    })
}

async fn perform_checkout(path: PathBuf, branch_name: String) -> Result<String, String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("git2 open: {e}"))?;
    GitEngine::checkout_branch(&repo, &branch_name).map_err(|e| format!("{e}"))?;
    Ok(branch_name)
}

async fn perform_commit(
    path: PathBuf,
    summary: String,
    description: String,
    files: Vec<WorkingDirectoryFileChange>,
) -> Result<String, String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("git2 open: {e}"))?;

    let context = CommitContext {
        summary,
        description: if description.trim().is_empty() {
            None
        } else {
            Some(description)
        },
        amend: false,
        co_authors: Vec::new(),
    };

    GitEngine::create_commit(&repo, &context, &files)
        .map(|oid| oid.to_string())
        .map_err(|e| format!("{e}"))
}

/// Open the repository on the async thread-pool, then compute the file diff.
/// `git2::Repository` is not `Send`, so we construct it inside the future.
async fn perform_diff(repo_path: PathBuf, file_path: PathBuf) -> Result<String, String> {
    let repo =
        git2::Repository::open(&repo_path).map_err(|e| format!("git2 open: {e}"))?;
    GitEngine::get_file_diff(&repo, &file_path).map_err(|e| format!("{e}"))
}