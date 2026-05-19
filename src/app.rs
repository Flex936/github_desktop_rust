//! Root application state and iced lifecycle for the GitHub Desktop Rust clone.
//!
//! Iced 0.14 paradigms used here:
//!   - Free-function `update` and `view` (no `Application` trait impl required).
//!   - `Task<Message>` returned from `update` for async work.
//!   - `Task::perform` to run the (blocking-wrapped) git2 calls off the main thread.
//!   - `iced::application(title, update, view).run_with(init)` for startup.

use std::path::PathBuf;

use iced::{
    Background, Color, Element, Length, Task,
    widget::{button, checkbox, column, container, pick_list, row, scrollable, text},
};

use crate::git_engine::GitEngine;
use crate::models::branch::Branch;
use crate::models::commit::Commit;
use crate::models::repository::Repository;
use crate::models::status::{DiffSelectionType, WorkingDirectoryStatus};

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

/// The top-level application state.
pub struct App {
    /// The path that was last passed to `LoadRepository`, retained so that
    /// `Message::Refresh` knows where to reload from.
    pub repo_path: Option<PathBuf>,
    /// The repository that is currently open, or `None` while loading.
    pub repository: Option<Repository>,
    /// Snapshot of the working-directory diff (staged / unstaged files).
    pub status: Option<WorkingDirectoryStatus>,
    /// Commit history (most-recent first, capped at `HISTORY_LIMIT`).
    pub commits: Vec<Commit>,
    /// All local and remote branches.
    pub branches: Vec<Branch>,
    /// The branch name currently selected in the branch picker dropdown.
    /// Starts as `None`; set on load (to the HEAD branch name) and on user picks.
    pub selected_branch: Option<String>,
    /// Set when the git engine returns an error; displayed in the UI.
    pub error: Option<String>,
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
        }
    }
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

/// The exhaustive set of events the application can react to.
#[derive(Debug, Clone)]
pub enum Message {
    /// Trigger an async load of every piece of git data for the given path.
    LoadRepository(PathBuf),
    /// Re-run the same load as the last `LoadRepository` call.
    /// No-ops silently when no repository has been opened yet.
    Refresh,
    /// Delivered once the async git load completes (success or error).
    DataLoaded(Result<GitSnapshot, String>),
    /// The user toggled the "select all / deselect all" checkbox in the
    /// Changes panel.  `true` = stage everything, `false` = unstage everything.
    ToggleAllFiles(bool),
    /// The user toggled the staging checkbox for a single file.
    /// The `String` is the stable `file_change.id()` key; `bool` is the new
    /// checked state (`true` = stage, `false` = unstage).
    ToggleFile(String, bool),
    /// The user selected a branch from the pick-list dropdown.
    /// Immediately kicks off an async `git checkout` via libgit2.
    BranchSelected(String),
    /// Delivered once the async checkout attempt finishes.
    ///
    /// On success the payload is the branch name that was checked out, so the
    /// update handler can keep the dropdown and the engine state in sync even
    /// if a concurrent refresh arrives before this message.
    /// On failure the payload is a human-readable error string.
    CheckoutCompleted(Result<String, String>),
}

/// Everything returned by a single async git data fetch.
///
/// Bundling them into one type keeps the `Message` enum tidy and ensures that
/// all four pieces of data arrive atomically.
#[derive(Debug, Clone)]
pub struct GitSnapshot {
    pub repository: Repository,
    pub status: WorkingDirectoryStatus,
    pub commits: Vec<Commit>,
    pub branches: Vec<Branch>,
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

/// Maximum number of commits loaded into memory at startup.
const HISTORY_LIMIT: usize = 100;

/// iced 0.14 update function — processes one `Message` and returns a `Task`.
pub fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ── Load ────────────────────────────────────────────────────────────
        Message::LoadRepository(path) => {
            app.repo_path = Some(path.clone());

            app.repository = None;
            app.status = None;
            app.commits.clear();
            app.branches.clear();
            app.selected_branch = None;
            app.error = None;

            Task::perform(fetch_git_snapshot(path), Message::DataLoaded)
        }

        // ── Refresh ─────────────────────────────────────────────────────────
        Message::Refresh => {
            match app.repo_path.clone() {
                Some(path) => {
                    app.repository = None;
                    app.status = None;
                    app.commits.clear();
                    app.branches.clear();
                    // Preserve the selected branch across refresh; DataLoaded
                    // will validate and reset it if the branch no longer exists.
                    app.error = None;

                    Task::perform(fetch_git_snapshot(path), Message::DataLoaded)
                }
                None => Task::none(),
            }
        }

        // ── Data arrived ────────────────────────────────────────────────────
        Message::DataLoaded(Ok(snapshot)) => {
            let prev_selection = app.selected_branch.take();
            let new_names: Vec<String> = snapshot.branches.iter().map(|b| b.name.clone()).collect();

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

        // ── Stage / unstage all files ────────────────────────────────────────
        Message::ToggleAllFiles(is_checked) => {
            if let Some(status) = app.status.take() {
                app.status = Some(status.with_include_all_files(is_checked));
            }
            Task::none()
        }

        // ── Stage / unstage a single file ────────────────────────────────────
        Message::ToggleFile(id, is_checked) => {
            if let Some(status) = app.status.take() {
                // Rebuild the file list: update the matching entry, leave
                // everything else as-is, then recompute `include_all` via
                // `WorkingDirectoryStatus::from_files`.
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

                // `from_files` recomputes the master `include_all` flag.
                app.status = Some(WorkingDirectoryStatus::from_files(new_files));
            }
            Task::none()
        }

        // ── Branch selected → kick off checkout ──────────────────────────────
        //
        // Optimistically update the dropdown immediately so the UI feels
        // responsive, then run the actual libgit2 checkout asynchronously.
        // If the checkout fails, CheckoutCompleted(Err) will revert the
        // selection and surface the error.
        Message::BranchSelected(name) => {
            // Bail out early if the user re-selected the branch that is already
            // checked out — no work to do.
            if app.selected_branch.as_deref() == Some(&name) {
                return Task::none();
            }

            // Optimistic UI update.
            app.selected_branch = Some(name.clone());
            app.error = None;

            let Some(path) = app.repo_path.clone() else {
                // No repo open yet; shouldn't be reachable from the UI, but
                // guard defensively.
                return Task::none();
            };

            Task::perform(perform_checkout(path, name), Message::CheckoutCompleted)
        }

        // ── Checkout result ──────────────────────────────────────────────────
        Message::CheckoutCompleted(Ok(branch_name)) => {
            // Confirm the optimistic selection (it may have changed if the user
            // clicked very fast, so we write it unconditionally).
            app.selected_branch = Some(branch_name);
            app.error = None;

            // Immediately re-fetch all git data so the commit history,
            // changed-files list, and branch list reflect the new HEAD.
            // This is equivalent to the TypeScript app calling
            // `this._updateRepository()` after a successful checkout.
            let Some(path) = app.repo_path.clone() else {
                return Task::none();
            };

            Task::perform(fetch_git_snapshot(path), Message::DataLoaded)
        }

        Message::CheckoutCompleted(Err(e)) => {
            // The checkout failed (e.g. dirty working tree conflicts with the
            // target branch).  Revert the optimistic selection so the dropdown
            // doesn't lie to the user.
            //
            // We leave `selected_branch` as-is; on the next DataLoaded (from a
            // background refresh) it will be corrected to whatever HEAD actually
            // is.  For now just show the error.
            app.error = Some(format!("Checkout failed: {e}"));
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Async git workers
// ---------------------------------------------------------------------------

/// Runs all four git2 operations and bundles the results.
async fn fetch_git_snapshot(path: PathBuf) -> Result<GitSnapshot, String> {
    let repository = GitEngine::open_repository(&path, 1).map_err(|e| format!("open: {e}"))?;

    let native = git2::Repository::open(&path).map_err(|e| format!("git2 open: {e}"))?;

    let status =
        GitEngine::get_working_directory_status(&native).map_err(|e| format!("status: {e}"))?;

    let commits = GitEngine::get_commit_history(&native, HISTORY_LIMIT)
        .map_err(|e| format!("history: {e}"))?;

    let branches = GitEngine::get_branches(&native).map_err(|e| format!("branches: {e}"))?;

    Ok(GitSnapshot {
        repository,
        status,
        commits,
        branches,
    })
}

/// Runs `GitEngine::checkout_branch` for `branch_name` inside the repo at
/// `path`.  Returns the branch name on success so the caller can confirm the
/// optimistic UI update.
///
/// This is intentionally a separate `async` fn (rather than inlining into the
/// `Task::perform` closure) so that the libgit2 calls are easy to unit-test
/// and so the borrow of `path` / `branch_name` is clear.
async fn perform_checkout(path: PathBuf, branch_name: String) -> Result<String, String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("git2 open: {e}"))?;

    GitEngine::checkout_branch(&repo, &branch_name).map_err(|e| format!("{e}"))?;

    Ok(branch_name)
}

// ---------------------------------------------------------------------------
// View helpers — colours
// ---------------------------------------------------------------------------

const SIDEBAR_BG: Color = Color {
    r: 0.141,
    g: 0.141,
    b: 0.149,
    a: 1.0,
};
const MAIN_BG: Color = Color {
    r: 0.196,
    g: 0.196,
    b: 0.204,
    a: 1.0,
};
const MUTED: Color = Color {
    r: 0.55,
    g: 0.55,
    b: 0.58,
    a: 1.0,
};
const WHITE: Color = Color {
    r: 0.95,
    g: 0.95,
    b: 0.96,
    a: 1.0,
};
const ERROR_RED: Color = Color {
    r: 0.85,
    g: 0.33,
    b: 0.33,
    a: 1.0,
};

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub fn view(app: &App) -> Element<Message> {
    let sidebar = view_sidebar(app);
    let main_content = view_main(app);

    row![sidebar, main_content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view_sidebar(app: &App) -> Element<Message> {
    let repo_name = app
        .repository
        .as_ref()
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "Loading…".into());

    let refresh_btn = button(text("↺ Refresh").size(11))
        .on_press_maybe(app.repo_path.as_ref().map(|_| Message::Refresh))
        .padding([3, 8]);

    let header = column![
        text("REPOSITORY").size(10).color(MUTED),
        row![
            text(repo_name).size(15).color(WHITE),
            iced::widget::Space::new(),
            refresh_btn,
        ]
        .align_y(iced::Alignment::Center),
    ]
    .spacing(4);

    let branch_header = text(format!("BRANCHES ({})", app.branches.len()))
        .size(10)
        .color(MUTED);

    // We filter the list so the dropdown ONLY shows local branches,
    // preventing the user from clicking remotes our checkout engine can't handle yet.
    let branch_names: Vec<String> = app
        .branches
        .iter()
        .filter(|b| b.branch_type == crate::models::branch::BranchType::Local)
        .map(|b| b.name.clone())
        .collect();

    let branch_picker: Element<Message> = if branch_names.is_empty() {
        text("No branches loaded").size(12).color(MUTED).into()
    } else {
        pick_list(
            branch_names,
            app.selected_branch.clone(),
            Message::BranchSelected,
        )
        .width(Length::Fill)
        .placeholder("Select a branch…")
        .into()
    };

    // ── Changes header with "toggle all" checkbox ──────────────────────────
    //
    // `include_all` is `Option<bool>`:
    //   Some(true)  → every file staged   → checkbox checked
    //   Some(false) → no file staged       → checkbox unchecked
    //   None        → partial selection    → treat as unchecked for iced's
    //                                        binary widget; user sees files
    //                                        individually to understand state
    let all_checked = app
        .status
        .as_ref()
        .and_then(|s| s.include_all)
        .unwrap_or(false);
    let toggle_all_cb = checkbox(all_checked)
        .label("CHANGES")
        .on_toggle(Message::ToggleAllFiles)
        .text_size(10)
        .size(14);

    // ── Per-file checkboxes ────────────────────────────────────────────────
    let file_rows: Element<Message> = match &app.status {
        None => text("No status loaded").size(12).color(MUTED).into(),
        Some(status) if status.files.is_empty() => {
            text("No changed files").size(12).color(MUTED).into()
        }
        Some(status) => {
            let rows: Vec<Element<Message>> = status
                .files
                .iter()
                .map(|wdf| {
                    let file_id = wdf.id();
                    let is_staged = wdf.selection.selection_type() == DiffSelectionType::All;
                    let label = wdf
                        .file_change
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| wdf.file_change.path.display().to_string());

                    checkbox(is_staged)
                        .label(label)
                        .on_toggle(move |checked| Message::ToggleFile(file_id.clone(), checked))
                        .text_size(11)
                        .size(13)
                        .into()
                })
                .collect();

            scrollable(column(rows).spacing(4))
                .height(Length::Fixed(180.0))
                .into()
        }
    };

    let error_section: Element<Message> = match &app.error {
        Some(msg) => column![
            text("ERROR").size(10).color(ERROR_RED),
            text(msg).size(11).color(ERROR_RED),
        ]
        .spacing(4)
        .into(),
        None => column![].into(),
    };

    let sidebar_content = column![
        header,
        divider(),
        branch_header,
        branch_picker,
        divider(),
        toggle_all_cb,
        file_rows,
        divider(),
        error_section,
    ]
    .spacing(12)
    .padding(16);

    container(sidebar_content)
        .width(Length::Fixed(250.0))
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(SIDEBAR_BG)),
            ..Default::default()
        })
        .into()
}

fn view_main(app: &App) -> Element<Message> {
    let branch_line = match &app.selected_branch {
        Some(name) => format!("Branch: {name}"),
        None => "Branch: —".into(),
    };

    let commit_header = text(format!("COMMIT HISTORY ({} commits)", app.commits.len()))
        .size(10)
        .color(MUTED);

    let commit_rows: Element<Message> = if app.commits.is_empty() {
        text("No commits loaded").size(13).color(MUTED).into()
    } else {
        let rows = app.commits.iter().map(|c| {
            let merge_tag = if c.is_merge_commit() { " ⑃" } else { "" };
            let line = format!(
                "{}  {}  {}{}",
                c.short_sha,
                c.author.date.format("%Y-%m-%d"),
                c.summary,
                merge_tag,
            );
            text(line).size(12).color(WHITE).into()
        });
        iced::widget::scrollable(column(rows).spacing(6))
            .height(Length::Fill)
            .into()
    };

    let main_content = column![
        text("Main Content").size(18).color(WHITE),
        text(branch_line).size(12).color(MUTED),
        divider(),
        commit_header,
        commit_rows,
    ]
    .spacing(12)
    .padding(20);

    container(main_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(MAIN_BG)),
            ..Default::default()
        })
        .into()
}

fn divider<'a>() -> Element<'a, Message> {
    container(text(""))
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(|_| container::Style {
            background: Some(Background::Color(Color {
                r: 0.30,
                g: 0.30,
                b: 0.32,
                a: 1.0,
            })),
            ..Default::default()
        })
        .into()
}
