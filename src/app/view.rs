use iced::{
    Background, Border, Color, Element, Length,
    widget::{button, checkbox, column, container, pick_list, row, scrollable, text, text_input},
};

use crate::app::{App, Message};
use crate::models::status::DiffSelectionType;

// ── Colour palette ────────────────────────────────────────────────────────────

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
/// The GitHub-purple used for the active commit button.
const ACCENT: Color = Color {
    r: 0.22,
    g: 0.52,
    b: 0.96,
    a: 1.0,
};
/// A dimmed version of ACCENT for the disabled state.
const ACCENT_DIM: Color = Color {
    r: 0.22,
    g: 0.35,
    b: 0.55,
    a: 1.0,
};

// ── Top-level view ────────────────────────────────────────────────────────────

pub fn view(app: &App) -> Element<Message> {
    let sidebar = view_sidebar(app);
    let main_content = view_main(app);

    row![sidebar, main_content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

fn view_sidebar(app: &App) -> Element<Message> {
    // ── Repository header ─────────────────────────────────────────────────
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

    // --- Branch picker ---
    // 1. First, filter the branches to ONLY local branches
    let local_branches: Vec<String> = app
        .branches
        .iter()
        .filter(|b| b.branch_type == crate::models::branch::BranchType::Local)
        .map(|b| b.name.clone())
        .collect();

    // 2. Use the length of the FILTERED list for the header
    let branch_header = text(format!("BRANCHES ({})", local_branches.len()))
        .size(10)
        .color(MUTED);

    // 3. Pass the filtered list to the dropdown
    let branch_picker: Element<Message> = if local_branches.is_empty() {
        text("No branches loaded").size(12).color(MUTED).into()
    } else {
        pick_list(
            local_branches,
            app.selected_branch.clone(),
            Message::BranchSelected,
        )
        .width(Length::Fill)
        .placeholder("Select a branch…")
        .into()
    };

    // ── Changed-files list ────────────────────────────────────────────────
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

    // ── Error banner ──────────────────────────────────────────────────────
    let error_section: Element<Message> = match &app.error {
        Some(msg) => column![
            text("ERROR").size(10).color(ERROR_RED),
            text(msg).size(11).color(ERROR_RED),
        ]
        .spacing(4)
        .into(),
        None => column![].into(),
    };

    // ── Commit panel ──────────────────────────────────────────────────────
    //
    // Summary (required) — single-line text input.
    let summary_input = text_input("Summary (required)", &app.commit_summary)
        .on_input(Message::CommitSummaryChanged)
        .padding([5, 8])
        .size(12)
        .width(Length::Fill);

    // Description (optional) — second single-line input used as a simple
    // multi-line proxy until iced gains a native multi-line text area.
    let description_input = text_input("Description (optional)", &app.commit_description)
        .on_input(Message::CommitDescriptionChanged)
        .padding([5, 8])
        .size(12)
        .width(Length::Fill);

    // "Commit to <branch>" button — enabled only when the summary is non-empty
    // and there is at least one file in the working directory.
    let branch_label = app.selected_branch.as_deref().unwrap_or("branch");

    let has_files = app
        .status
        .as_ref()
        .map(|s| !s.files.is_empty())
        .unwrap_or(false);
    let can_commit = !app.commit_summary.trim().is_empty() && has_files;

    let commit_btn = button(
        text(format!("Commit to {branch_label}"))
            .size(12)
            .color(WHITE),
    )
    .on_press_maybe(if can_commit {
        Some(Message::Commit)
    } else {
        None
    })
    .width(Length::Fill)
    .padding([7, 12])
    .style(move |_theme, _status| {
        let bg = if can_commit { ACCENT } else { ACCENT_DIM };
        button::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            text_color: WHITE,
            ..Default::default()
        }
    });

    let commit_panel = column![
        text("COMMIT").size(10).color(MUTED),
        summary_input,
        description_input,
        commit_btn,
    ]
    .spacing(6);

    // ── Assemble sidebar ──────────────────────────────────────────────────
    let sidebar_content = column![
        header,
        divider(),
        branch_header,
        branch_picker,
        divider(),
        toggle_all_cb,
        file_rows,
        divider(),
        commit_panel,
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

// ── Main content ──────────────────────────────────────────────────────────────

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

// ── Helpers ───────────────────────────────────────────────────────────────────

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
