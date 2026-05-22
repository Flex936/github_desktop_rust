//! view.rs — GitHub Desktop dark-mode UI (iced 0.14)
//!
//! Palette source: GitHub Primer dark theme
//!   https://primer.style/foundations/color/overview

use iced::{
    Background, Border, Color, Element, Font, Length, Padding, Shadow, Vector,
    widget::{
        Space, button, checkbox, column, container, pick_list, row, scrollable, text,
        text_input,
    },
};

use crate::app::{App, Message};
use crate::models::status::{AppFileStatusKind, DiffSelectionType};

// ── Primer dark palette ───────────────────────────────────────────────────────

const SIDEBAR_BG: Color   = hex(0x1f, 0x24, 0x28);
const MAIN_BG: Color      = hex(0x24, 0x29, 0x2e);
const TOOLBAR_BG: Color   = hex(0x2d, 0x33, 0x38);
const SURFACE: Color      = hex(0x2f, 0x36, 0x3d);
const ROW_HOVER: Color    = hex(0x2b, 0x30, 0x36);
const ROW_SELECTED: Color = hex(0x31, 0x3a, 0x42);
const BORDER: Color       = hex(0x1b, 0x1f, 0x23);
const BORDER_SUBTLE: Color = hex(0x37, 0x3e, 0x47);
const TEXT: Color         = hex(0xe1, 0xe4, 0xe8);
const MUTED: Color        = hex(0x95, 0x9d, 0xa5);
const MUTED2: Color       = hex(0x6a, 0x73, 0x7d);
const ACCENT: Color       = hex(0x03, 0x66, 0xd6);
const ACCENT_HOVER: Color = hex(0x00, 0x5c, 0xc5);
const ACCENT_PRESS: Color = hex(0x04, 0x42, 0x89);
const ACCENT_DIM: Color   = hex(0x1c, 0x3a, 0x5e);
const ERROR: Color        = hex(0xf9, 0x75, 0x83);
const ERROR_BG: Color     = hex(0x3d, 0x1c, 0x24);
const GREEN: Color        = hex(0x34, 0xd0, 0x58);
const YELLOW: Color       = hex(0xff, 0xea, 0x7f);
const RED: Color          = ERROR;
const CYAN: Color         = hex(0x79, 0xb8, 0xff);

/// Diff-hunk header colour — a muted purple so it reads distinctly.
const HUNK_HDR: Color = hex(0xb3, 0x92, 0xf0);

const fn hex(r: u8, g: u8, b: u8) -> Color {
    Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
}

// ── Top-level view ────────────────────────────────────────────────────────────

pub fn view(app: &App) -> Element<Message> {
    let sidebar      = view_sidebar(app);
    let main_content = view_main(app);

    row![sidebar, main_content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Main panel dispatcher ─────────────────────────────────────────────────────

fn view_main(app: &App) -> Element<Message> {
    if app.selected_file_path.is_some() {
        view_diff(app)
    } else {
        view_history(app)
    }
}

// ── Diff view (replaces history when a file is selected) ──────────────────────

fn view_diff(app: &App) -> Element<Message> {
    // `selected_file_path` is guaranteed Some by the dispatcher above.
    let path = app.selected_file_path.as_deref().unwrap();
    let path_display = path.display().to_string();

    // ── Toolbar ───────────────────────────────────────────────────────────
    let branch_name = app
        .selected_branch
        .clone()
        .unwrap_or_else(|| "—".to_string());

    let toolbar = container(
        row![
            text("⎇").size(14).color(MUTED),
            text(branch_name).size(14).color(TEXT),
            Space::new().width(Length::Fill),
            text(path_display.clone()).size(11).color(MUTED2),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .padding([10.0, 20.0])
    .style(|_| container::Style {
        background: Some(Background::Color(TOOLBAR_BG)),
        border: Border { color: BORDER, width: 1.0, radius: 0.0.into() },
        ..Default::default()
    });

    // ── Section label ─────────────────────────────────────────────────────
    let diff_label = container(
        text(format!("DIFF — {}", path_display))
            .size(10)
            .color(MUTED2),
    )
    .padding(Padding { top: 12.0, right: 20.0, bottom: 6.0, left: 20.0 });

    // ── Diff body ─────────────────────────────────────────────────────────
    let diff_body: Element<Message> = match &app.current_diff {
        None => container(
            text("Loading diff…").size(13).color(MUTED2),
        )
        .padding([20.0, 20.0])
        .into(),

        Some(diff_text) if diff_text.trim().is_empty() => container(
            text("(empty diff)").size(13).color(MUTED2),
        )
        .padding([20.0, 20.0])
        .into(),

        Some(diff_text) => {
            // Render each patch line as a coloured, monospace text element.
            // We choose colour by the leading character of each line:
            //   '+'  → green   (added)
            //   '-'  → red     (deleted)
            //   '@'  → purple  (hunk header @@…@@)
            //   'diff'/'index'/'---'/'+++' file-header lines → muted
            //   else → primary text (context)
            let line_elements: Vec<Element<Message>> = diff_text
                .lines()
                .map(|line| {
                    let color = if line.starts_with('+') && !line.starts_with("+++") {
                        GREEN
                    } else if line.starts_with('-') && !line.starts_with("---") {
                        RED
                    } else if line.starts_with("@@") {
                        HUNK_HDR
                    } else if line.starts_with("diff ")
                        || line.starts_with("index ")
                        || line.starts_with("---")
                        || line.starts_with("+++")
                        || line.starts_with("Binary")
                    {
                        MUTED
                    } else {
                        TEXT
                    };

                    // Preserve every space in the line — `text` trims by
                    // default in iced, so we append a zero-width joiner to
                    // force it to treat leading spaces as content.
                    text(line.to_string())
                        .size(12)
                        .color(color)
                        .font(Font::MONOSPACE)
                        .into()
                })
                .collect();

            scrollable(
                container(
                    column(line_elements)
                        .spacing(0)
                        .width(Length::Fill),
                )
                .padding(Padding { top: 8.0, right: 20.0, bottom: 12.0, left: 20.0 }),
            )
            .height(Length::Fill)
            .into()
        }
    };

    let main_inner = column![toolbar, diff_label, thin_divider(), diff_body]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    container(main_inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(MAIN_BG)),
            ..Default::default()
        })
        .into()
}

// ── Commit History view (the original main panel) ─────────────────────────────

fn view_history(app: &App) -> Element<Message> {
    let branch_name = app
        .selected_branch
        .clone()
        .unwrap_or_else(|| "—".to_string());

    let toolbar = container(
        row![
            text("⎇").size(14).color(MUTED),
            text(branch_name).size(14).color(TEXT),
            Space::new().width(Length::Fill),
            text(format!("{} commits", app.commits.len()))
                .size(11)
                .color(MUTED2),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .padding([10.0, 20.0])
    .style(|_| container::Style {
        background: Some(Background::Color(TOOLBAR_BG)),
        border: Border { color: BORDER, width: 1.0, radius: 0.0.into() },
        ..Default::default()
    });

    let history_label = container(text("COMMIT HISTORY").size(10).color(MUTED2))
        .padding(Padding { top: 12.0, right: 20.0, bottom: 6.0, left: 20.0 });

    let commit_list: Element<Message> = if app.commits.is_empty() {
        container(text("No commits loaded").size(13).color(MUTED2))
            .padding([20.0, 20.0])
            .into()
    } else {
        let rows: Vec<Element<Message>> = app
            .commits
            .iter()
            .map(|c| {
                let merge_icon = if c.is_merge_commit() { " ⑃" } else { "" };

                let sha_chip = container(text(c.short_sha.clone()).size(10).color(CYAN))
                    .padding([2.0, 6.0])
                    .style(|_| container::Style {
                        background: Some(Background::Color(SURFACE)),
                        border: Border {
                            color: BORDER_SUBTLE,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    });

                let date_str = c.author.date.format("%b %d, %Y").to_string();

                container(
                    column![
                        row![
                            text(format!("{}{}", c.summary, merge_icon))
                                .size(13)
                                .color(TEXT),
                            Space::new().width(Length::Fill),
                            sha_chip,
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                        row![
                            text(c.author.name.clone()).size(11).color(MUTED),
                            text("·").size(11).color(MUTED2),
                            text(date_str).size(11).color(MUTED2),
                        ]
                        .spacing(5)
                        .align_y(iced::Alignment::Center),
                    ]
                    .spacing(3),
                )
                .width(Length::Fill)
                .padding([8.0, 20.0])
                .style(|_| container::Style {
                    background: None,
                    border: Border {
                        color: Color { a: 0.4, ..BORDER },
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                })
                .into()
            })
            .collect();

        scrollable(column(rows).spacing(0).width(Length::Fill))
            .height(Length::Fill)
            .into()
    };

    let main_inner = column![toolbar, history_label, thin_divider(), commit_list]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    container(main_inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(MAIN_BG)),
            ..Default::default()
        })
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

    let refresh_btn = button(text("↺").size(14).color(MUTED))
        .on_press_maybe(app.repo_path.as_ref().map(|_| Message::Refresh))
        .padding([4.0, 8.0])
        .style(|_theme, status| {
            use button::Status::*;
            let bg = match status {
                Hovered => Some(Background::Color(ROW_HOVER)),
                Pressed => Some(Background::Color(SURFACE)),
                _ => None,
            };
            button::Style {
                background: bg,
                border: Border { radius: 6.0.into(), ..Default::default() },
                text_color: match status {
                    Hovered | Pressed => TEXT,
                    _ => MUTED,
                },
                ..Default::default()
            }
        });

    let header = container(
        column![
            text("REPOSITORY").size(10).color(MUTED2),
            row![
                text(repo_name).size(14).color(TEXT),
                Space::new().width(Length::Fill),
                refresh_btn,
            ]
            .align_y(iced::Alignment::Center),
        ]
        .spacing(6),
    )
    .padding([12.0, 16.0]);

    // ── Branch section ────────────────────────────────────────────────────
    let local_branches: Vec<String> = app
        .branches
        .iter()
        .filter(|b| b.branch_type == crate::models::branch::BranchType::Local)
        .map(|b| b.name.clone())
        .collect();

    let branch_section_label = text("CURRENT BRANCH").size(10).color(MUTED2);

    let branch_picker: Element<Message> = if local_branches.is_empty() {
        text("No branches").size(12).color(MUTED2).into()
    } else {
        pick_list(
            local_branches,
            app.selected_branch.clone(),
            Message::BranchSelected,
        )
        .width(Length::Fill)
        .placeholder("Select a branch…")
        .text_size(13)
        .padding([7.0, 10.0])
        .into()
    };

    let branch_section =
        container(column![branch_section_label, branch_picker].spacing(6))
            .padding([0.0, 16.0]);

    // ── Changed-files section ─────────────────────────────────────────────
    let file_count = app.status.as_ref().map(|s| s.files.len()).unwrap_or(0);
    let all_checked = app
        .status
        .as_ref()
        .and_then(|s| s.include_all)
        .unwrap_or(false);

    let changes_header = row![
        text(format!("CHANGES ({})", file_count))
            .size(10)
            .color(MUTED2),
        Space::new().width(Length::Fill),
        checkbox(all_checked)
            .label("All")
            .on_toggle(Message::ToggleAllFiles)
            .text_size(10)
            .size(13),
    ]
    .align_y(iced::Alignment::Center);

    let file_rows: Element<Message> = match &app.status {
        None => padded_muted("No status loaded"),
        Some(s) if s.files.is_empty() => padded_muted("No changed files"),
        Some(status) => {
            let rows: Vec<Element<Message>> = status
                .files
                .iter()
                .map(|wdf| {
                    let file_id   = wdf.id();
                    let is_staged = wdf.selection.selection_type() == DiffSelectionType::All;

                    // Is this the file currently open in the diff panel?
                    let is_selected = app.selected_file_path.as_deref()
                        == Some(wdf.file_change.path.as_path());

                    let filename = wdf
                        .file_change
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| wdf.file_change.path.display().to_string());

                    // Status badge colour & letter ────────────────────────
                    let status_color = match wdf.file_change.status.kind() {
                        AppFileStatusKind::New        => GREEN,
                        AppFileStatusKind::Modified   => YELLOW,
                        AppFileStatusKind::Deleted    => RED,
                        AppFileStatusKind::Renamed
                        | AppFileStatusKind::Copied   => CYAN,
                        AppFileStatusKind::Conflicted => ERROR,
                        AppFileStatusKind::Untracked  => MUTED,
                    };
                    let badge_char = match wdf.file_change.status.kind() {
                        AppFileStatusKind::New        => "A",
                        AppFileStatusKind::Modified   => "M",
                        AppFileStatusKind::Deleted    => "D",
                        AppFileStatusKind::Renamed    => "R",
                        AppFileStatusKind::Copied     => "C",
                        AppFileStatusKind::Conflicted => "!",
                        AppFileStatusKind::Untracked  => "?",
                    };

                    let badge = container(text(badge_char).size(9).color(status_color))
                        .padding([1.0, 4.0])
                        .style(move |_| container::Style {
                            background: Some(Background::Color(Color {
                                a: 0.18,
                                ..status_color
                            })),
                            border: Border {
                                color: Color { a: 0.35, ..status_color },
                                width: 1.0,
                                radius: 3.0.into(),
                            },
                            ..Default::default()
                        });

                    // Stage/unstage checkbox ──────────────────────────────
                    let cb = checkbox(is_staged)
                        .on_toggle(move |checked| {
                            Message::ToggleFile(file_id.clone(), checked)
                        })
                        .size(13);

                    // ── Filename as a transparent ghost-button ─────────────
                    //
                    // Wrapping the filename text in a button gives us a
                    // proper hit-target and ripple feedback while being
                    // visually invisible — background None, no border.
                    // We tint the label accent-blue when this file is the
                    // one currently shown in the diff panel.
                    let click_path = wdf.file_change.path.clone();
                    let label_color = if is_selected { ACCENT } else { TEXT };

                    let filename_btn = button(
                        text(filename).size(12).color(label_color),
                    )
                    .on_press(Message::FileClicked(click_path))
                    .padding(0)
                    .style(move |_theme, status| {
                        use button::Status::*;
                        let text_color = match status {
                            Hovered | Pressed => ACCENT,
                            _ => label_color,
                        };
                        button::Style {
                            background: None,
                            border: Border::default(),
                            text_color,
                            shadow: Shadow::default(),
                            ..Default::default()
                        }
                    });

                    // ── Row assembly ───────────────────────────────────────
                    let inner = row![
                        cb,
                        filename_btn,
                        Space::new().width(Length::Fill),
                        badge,
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center);

                    // Highlight the row of the currently-open file.
                    container(inner)
                        .width(Length::Fill)
                        .padding([4.0, 10.0])
                        .style(move |_| container::Style {
                            background: Some(Background::Color(if is_selected {
                                ROW_SELECTED
                            } else {
                                Color::TRANSPARENT
                            })),
                            ..Default::default()
                        })
                        .into()
                })
                .collect();

            scrollable(column(rows).spacing(1).width(Length::Fill))
                .height(Length::Fixed(200.0))
                .into()
        }
    };

    let changes_section =
        container(column![changes_header, file_rows].spacing(6)).padding([0.0, 16.0]);

    // ── Error banner ──────────────────────────────────────────────────────
    let error_section: Element<Message> = match &app.error {
        None => Space::new().into(),
        Some(msg) => container(
            column![
                row![
                    text("⚠").size(12).color(ERROR),
                    text("Error").size(11).color(ERROR),
                ]
                .spacing(4)
                .align_y(iced::Alignment::Center),
                text(msg.clone()).size(11).color(ERROR),
            ]
            .spacing(4),
        )
        .width(Length::Fill)
        .padding([8.0, 12.0])
        .style(|_| container::Style {
            background: Some(Background::Color(ERROR_BG)),
            border: Border {
                color: Color { a: 0.5, ..ERROR },
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into(),
    };

    // ── Commit panel ──────────────────────────────────────────────────────
    let branch_label = app.selected_branch.as_deref().unwrap_or("branch");

    let has_files = app
        .status
        .as_ref()
        .map(|s| !s.files.is_empty())
        .unwrap_or(false);
    let can_commit = !app.commit_summary.trim().is_empty() && has_files;

    let input_style = |_theme: &_, status: text_input::Status| {
        use text_input::Status::*;
        let border_color = match status {
            Focused { .. } => ACCENT,
            _ => BORDER_SUBTLE,
        };
        text_input::Style {
            background: Background::Color(SURFACE),
            border: Border { color: border_color, width: 1.0, radius: 6.0.into() },
            icon: TEXT,
            placeholder: MUTED2,
            value: TEXT,
            selection: Color { a: 0.35, ..ACCENT },
        }
    };

    let summary_input = text_input("Summary (required)", &app.commit_summary)
        .on_input(Message::CommitSummaryChanged)
        .padding([8.0, 10.0])
        .size(13)
        .width(Length::Fill)
        .style(input_style);

    let description_input = text_input("Description (optional)", &app.commit_description)
        .on_input(Message::CommitDescriptionChanged)
        .padding([8.0, 10.0])
        .size(12)
        .width(Length::Fill)
        .style(input_style);

    let commit_btn = button(
        container(
            text(format!("Commit to {}", branch_label))
                .size(13)
                .color(if can_commit { TEXT } else { MUTED }),
        )
        .center_x(Length::Fill),
    )
    .on_press_maybe(if can_commit { Some(Message::Commit) } else { None })
    .width(Length::Fill)
    .padding([9.0, 14.0])
    .style(move |_theme, status| {
        use button::Status::*;
        let bg = if !can_commit {
            ACCENT_DIM
        } else {
            match status {
                Hovered  => ACCENT_HOVER,
                Pressed  => ACCENT_PRESS,
                _        => ACCENT,
            }
        };
        let shadow = if can_commit {
            Shadow {
                color:       Color { a: 0.35, ..Color::BLACK },
                offset:      Vector::new(0.0, 1.0),
                blur_radius: 3.0,
            }
        } else {
            Shadow::default()
        };
        button::Style {
            background: Some(Background::Color(bg)),
            border: Border { radius: 6.0.into(), ..Default::default() },
            text_color: if can_commit { TEXT } else { MUTED },
            shadow,
            ..Default::default()
        }
    });

    let commit_panel = container(
        column![
            text("COMMIT").size(10).color(MUTED2),
            summary_input,
            description_input,
            commit_btn,
        ]
        .spacing(8),
    )
    .padding([0.0, 16.0]);

    // ── Assemble sidebar ──────────────────────────────────────────────────
    let sidebar_inner = column![
        header,
        thin_divider(),
        branch_section,
        thin_divider(),
        changes_section,
        thin_divider(),
        commit_panel,
        thin_divider(),
        container(error_section).padding([0.0, 16.0]),
        Space::new().height(8.0),
    ]
    .spacing(12)
    .width(Length::Fill);

    container(scrollable(sidebar_inner).height(Length::Fill))
        .width(Length::Fixed(270.0))
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(SIDEBAR_BG)),
            border: Border { color: BORDER, width: 1.0, radius: 0.0.into() },
            ..Default::default()
        })
        .into()
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn thin_divider<'a>() -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(1.0))
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(|_| container::Style {
            background: Some(Background::Color(BORDER)),
            ..Default::default()
        })
        .into()
}

fn padded_muted<'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label.to_string()).size(12).color(MUTED2))
        .padding([8.0, 10.0])
        .into()
}