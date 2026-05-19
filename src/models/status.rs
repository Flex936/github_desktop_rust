//! Translations of:
//!   status.ts        →  all file-change and working-directory types
//!   diff-selection.ts →  DiffSelection (embedded here; extract to diff.rs later)

use std::collections::HashSet;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// GitStatusEntry  (raw codes reported by `git status --porcelain=v2`)
// ---------------------------------------------------------------------------

/// The raw one-character status codes that Git uses in porcelain output.
///
/// Maps to the TypeScript `GitStatusEntry` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStatusEntry {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Unchanged,
    Untracked,
    Ignored,
    UpdatedButUnmerged,
}

impl GitStatusEntry {
    /// Parse the single character Git uses in `--porcelain` output.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'M' => Some(Self::Modified),
            'A' => Some(Self::Added),
            'D' => Some(Self::Deleted),
            'R' => Some(Self::Renamed),
            'C' => Some(Self::Copied),
            '.' => Some(Self::Unchanged),
            '?' => Some(Self::Untracked),
            '!' => Some(Self::Ignored),
            'U' => Some(Self::UpdatedButUnmerged),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// AppFileStatusKind  (Desktop's own file-status taxonomy)
// ---------------------------------------------------------------------------

/// How Desktop classifies a changed file.
///
/// Maps to the TypeScript `AppFileStatusKind` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppFileStatusKind {
    New,
    Modified,
    Deleted,
    Copied,
    Renamed,
    Conflicted,
    Untracked,
}

// ---------------------------------------------------------------------------
// SubmoduleStatus
// ---------------------------------------------------------------------------

/// The state of a submodule within a repository.
///
/// Maps to the TypeScript `SubmoduleStatus` type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmoduleStatus {
    /// The submodule is pointing at a different commit than the parent records.
    pub commit_changed: bool,
    /// The submodule has uncommitted modifications.
    pub modified_changes: bool,
    /// The submodule has untracked files.
    pub untracked_changes: bool,
}

// ---------------------------------------------------------------------------
// AppFileStatus variants
// ---------------------------------------------------------------------------

/// A straightforward file addition, modification, or deletion.
///
/// Maps to `PlainFileStatus`.
#[derive(Debug, Clone)]
pub struct PlainFileStatus {
    /// Must be `New`, `Modified`, or `Deleted`.
    pub kind: AppFileStatusKind,
    pub submodule_status: Option<SubmoduleStatus>,
}

/// A file that was copied or renamed — carries the original path.
///
/// Maps to `CopiedOrRenamedFileStatus`.
#[derive(Debug, Clone)]
pub struct CopiedOrRenamedFileStatus {
    /// Must be `Copied` or `Renamed`.
    pub kind: AppFileStatusKind,
    pub old_path: PathBuf,
    pub submodule_status: Option<SubmoduleStatus>,
}

/// The possible merge-conflict scenarios Desktop surfaces to the user.
///
/// Maps to `UnmergedEntrySummary`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmergedEntrySummary {
    AddedByUs,
    DeletedByUs,
    AddedByThem,
    DeletedByThem,
    BothDeleted,
    BothAdded,
    BothModified,
}

/// A file that is marked conflicted in the index.
///
/// When `conflict_marker_count` is `Some`, Desktop can inspect the inline
/// conflict markers (maps to `ConflictsWithMarkers`).  When it is `None`,
/// the user must pick "ours" or "theirs" manually (maps to `ManualConflict`).
#[derive(Debug, Clone)]
pub struct ConflictedFileStatus {
    pub action: UnmergedEntrySummary,
    /// The number of `<<<<<<<` / `=======` / `>>>>>>>` marker sets found.
    /// `None` means this is a manual (binary / mode-change) conflict.
    pub conflict_marker_count: Option<u32>,
    pub submodule_status: Option<SubmoduleStatus>,
}

impl ConflictedFileStatus {
    /// `true` when the conflict can be resolved by editing inline markers.
    pub fn has_markers(&self) -> bool {
        self.conflict_marker_count.is_some()
    }
}

/// An untracked file in the working directory.
///
/// Maps to `UntrackedFileStatus`.
#[derive(Debug, Clone)]
pub struct UntrackedFileStatus {
    pub submodule_status: Option<SubmoduleStatus>,
}

/// The complete set of possible file states Desktop can encounter.
///
/// Maps to the TypeScript `AppFileStatus` union type.
#[derive(Debug, Clone)]
pub enum AppFileStatus {
    Plain(PlainFileStatus),
    CopiedOrRenamed(CopiedOrRenamedFileStatus),
    Conflicted(ConflictedFileStatus),
    Untracked(UntrackedFileStatus),
}

impl AppFileStatus {
    pub fn kind(&self) -> AppFileStatusKind {
        match self {
            Self::Plain(s) => s.kind,
            Self::CopiedOrRenamed(s) => s.kind,
            Self::Conflicted(_) => AppFileStatusKind::Conflicted,
            Self::Untracked(_) => AppFileStatusKind::Untracked,
        }
    }

    pub fn is_conflicted(&self) -> bool {
        matches!(self, Self::Conflicted(_))
    }
}

// ---------------------------------------------------------------------------
// DiffSelection  (diff-selection.ts, embedded here for simplicity)
// ---------------------------------------------------------------------------
//
// The TypeScript version is a sophisticated immutable data structure; this
// Rust translation preserves that immutability via owned return values and
// uses a `HashSet<usize>` for the diverging-lines set.

/// Whether all, some, or no lines in a diff are included in the next commit.
///
/// Maps to `DiffSelectionType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSelectionType {
    /// Every line is selected.
    All,
    /// A subset of lines is selected.
    Partial,
    /// No lines are selected.
    None,
}

/// Tracks which lines of a diff are staged for the next commit.
///
/// Starts with a uniform default state (`All` or `None`) and records only the
/// lines that diverge from that default, keeping memory use proportional to the
/// size of the partial selection rather than the total line count.
///
/// Maps to the TypeScript `DiffSelection` class.
#[derive(Debug, Clone)]
pub struct DiffSelection {
    /// The default state every line starts in.
    default_type: DiffSelectionType, // always All or None
    /// Lines whose selection state differs from `default_type`.
    diverging_lines: Option<HashSet<usize>>,
    /// If known, the complete set of lines that *can* be selected (context /
    /// hunk-header lines cannot be staged individually).
    selectable_lines: Option<HashSet<usize>>,
}

impl DiffSelection {
    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Create a selection where every line is selected.
    pub fn select_all() -> Self {
        Self {
            default_type: DiffSelectionType::All,
            diverging_lines: None,
            selectable_lines: None,
        }
    }

    /// Create a selection where no lines are selected.
    pub fn select_none() -> Self {
        Self {
            default_type: DiffSelectionType::None,
            diverging_lines: None,
            selectable_lines: None,
        }
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// The computed overall selection state of this diff.
    pub fn selection_type(&self) -> DiffSelectionType {
        let diverging = match &self.diverging_lines {
            None => return self.default_type,
            Some(d) if d.is_empty() => return self.default_type,
            Some(d) => d,
        };

        // If we know the full selectable set and every selectable line diverges,
        // the effective state is the *opposite* of the default.
        if let Some(selectable) = &self.selectable_lines {
            if selectable.len() == diverging.len()
                && selectable.iter().all(|l| diverging.contains(l))
            {
                return match self.default_type {
                    DiffSelectionType::All => DiffSelectionType::None,
                    DiffSelectionType::None => DiffSelectionType::All,
                    DiffSelectionType::Partial => unreachable!(),
                };
            }
        }

        DiffSelectionType::Partial
    }

    /// Whether the line at `index` is currently selected.
    pub fn is_selected(&self, index: usize) -> bool {
        let divergent = self
            .diverging_lines
            .as_ref()
            .map(|d| d.contains(&index))
            .unwrap_or(false);

        match self.default_type {
            DiffSelectionType::All => !divergent,
            DiffSelectionType::None => divergent,
            DiffSelectionType::Partial => unreachable!(),
        }
    }

    /// Whether the line at `index` can be individually staged.
    pub fn is_selectable(&self, index: usize) -> bool {
        self.selectable_lines
            .as_ref()
            .map(|s| s.contains(&index))
            .unwrap_or(true)
    }

    // -----------------------------------------------------------------------
    // Transformations (return a new `DiffSelection`)
    // -----------------------------------------------------------------------

    /// Return a copy with `index` set to `selected`.
    pub fn with_line_selection(&self, index: usize, selected: bool) -> Self {
        self.with_range_selection(index, 1, selected)
    }

    /// Return a copy with `[from, from + length)` all set to `selected`.
    pub fn with_range_selection(&self, from: usize, length: usize, selected: bool) -> Self {
        if length == 0 {
            return self.clone();
        }

        let current = self.selection_type();

        // Nothing to do when the requested state already matches globally.
        let already_matches = match (current, selected) {
            (DiffSelectionType::All, true) => true,
            (DiffSelectionType::None, false) => true,
            _ => false,
        };
        if already_matches {
            return self.clone();
        }

        let to = from + length;

        let new_diverging = if current == DiffSelectionType::Partial {
            let mut lines = self.diverging_lines.clone().unwrap_or_default();
            let default_matches_selected = match (self.default_type, selected) {
                (DiffSelectionType::All, true) => true,
                (DiffSelectionType::None, false) => true,
                _ => false,
            };
            if default_matches_selected {
                // Removing divergence restores the default state.
                for i in from..to {
                    lines.remove(&i);
                }
            } else {
                for i in from..to {
                    if self.is_selectable(i) {
                        lines.insert(i);
                    }
                }
            }
            if lines.is_empty() { None } else { Some(lines) }
        } else {
            // Transitioning from a uniform state → need to record the range.
            let mut lines = HashSet::new();
            for i in from..to {
                if self.is_selectable(i) {
                    lines.insert(i);
                }
            }
            Some(lines)
        };

        Self {
            default_type: self.default_type,
            diverging_lines: new_diverging,
            selectable_lines: self.selectable_lines.clone(),
        }
    }

    /// Return a copy with the selection of `index` inverted.
    pub fn with_toggle_line(&self, index: usize) -> Self {
        let currently = self.is_selected(index);
        self.with_line_selection(index, !currently)
    }

    /// Return a copy where every line is selected.
    pub fn with_select_all(&self) -> Self {
        Self {
            default_type: DiffSelectionType::All,
            diverging_lines: None,
            selectable_lines: self.selectable_lines.clone(),
        }
    }

    /// Return a copy where no lines are selected.
    pub fn with_select_none(&self) -> Self {
        Self {
            default_type: DiffSelectionType::None,
            diverging_lines: None,
            selectable_lines: self.selectable_lines.clone(),
        }
    }

    /// Return a copy constrained to the given set of selectable line indices.
    pub fn with_selectable_lines(&self, selectable: HashSet<usize>) -> Self {
        // Drop any diverging lines that are no longer selectable.
        let diverging = self.diverging_lines.as_ref().map(|d| {
            d.iter()
                .copied()
                .filter(|l| selectable.contains(l))
                .collect::<HashSet<_>>()
        });
        Self {
            default_type: self.default_type,
            diverging_lines: diverging.filter(|d: &HashSet<usize>| !d.is_empty()),
            selectable_lines: Some(selectable),
        }
    }
}

// ---------------------------------------------------------------------------
// FileChange hierarchy
// ---------------------------------------------------------------------------

/// A file change associated with a commit or working-directory entry.
///
/// Maps to the TypeScript `FileChange` class.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub status: AppFileStatus,
}

impl FileChange {
    /// A stable string ID for this change, used as a map key in the UI.
    pub fn id(&self) -> String {
        let kind = self.status.kind();
        if let AppFileStatus::CopiedOrRenamed(ref s) = self.status {
            format!(
                "{:?}+{}+{}",
                kind,
                self.path.display(),
                s.old_path.display()
            )
        } else {
            format!("{:?}+{}", kind, self.path.display())
        }
    }
}

/// A file change in the *working directory* that the user can partially stage.
///
/// Maps to `WorkingDirectoryFileChange`.
#[derive(Debug, Clone)]
pub struct WorkingDirectoryFileChange {
    pub file_change: FileChange,
    /// Which lines (if any) the user has chosen to include in the next commit.
    pub selection: DiffSelection,
}

impl WorkingDirectoryFileChange {
    /// Convenience: select or deselect every line at once.
    pub fn with_include_all(&self, include: bool) -> Self {
        let selection = if include {
            self.selection.with_select_all()
        } else {
            self.selection.with_select_none()
        };
        self.with_selection(selection)
    }

    /// Return a copy with a new `DiffSelection`.
    pub fn with_selection(&self, selection: DiffSelection) -> Self {
        Self {
            file_change: self.file_change.clone(),
            selection,
        }
    }

    pub fn id(&self) -> String {
        self.file_change.id()
    }
}

/// A file change recorded in a specific commit.
///
/// Maps to `CommittedFileChange`.
#[derive(Debug, Clone)]
pub struct CommittedFileChange {
    pub file_change: FileChange,
    /// The commit SHA (the *after* snapshot).
    pub commitish: String,
    /// The parent commit SHA (the *before* snapshot).
    pub parent_commitish: String,
}

// ---------------------------------------------------------------------------
// WorkingDirectoryStatus
// ---------------------------------------------------------------------------

/// A snapshot of all pending changes in a repository's working directory.
///
/// Maps to the TypeScript `WorkingDirectoryStatus` class.
#[derive(Debug, Clone)]
pub struct WorkingDirectoryStatus {
    pub files: Vec<WorkingDirectoryFileChange>,
    /// `Some(true)` → all files selected, `Some(false)` → none selected,
    /// `None` → partial selection.
    pub include_all: Option<bool>,
}

impl WorkingDirectoryStatus {
    /// Build a status from a list of files, computing `include_all` automatically.
    pub fn from_files(files: Vec<WorkingDirectoryFileChange>) -> Self {
        let include_all = compute_include_all(&files);
        Self { files, include_all }
    }

    /// Stage or un-stage every file.
    pub fn with_include_all_files(&self, include: bool) -> Self {
        let files = self
            .files
            .iter()
            .map(|f| f.with_include_all(include))
            .collect();
        Self {
            files,
            include_all: Some(include),
        }
    }

    /// Find the file whose `id()` matches, returning a reference.
    pub fn find_file_with_id(&self, id: &str) -> Option<&WorkingDirectoryFileChange> {
        self.files.iter().find(|f| f.id() == id)
    }

    /// Find the index of the file whose `id()` matches, or `None`.
    pub fn find_file_index_by_id(&self, id: &str) -> Option<usize> {
        self.files.iter().position(|f| f.id() == id)
    }
}

fn compute_include_all(files: &[WorkingDirectoryFileChange]) -> Option<bool> {
    if files.is_empty() {
        return Some(true);
    }
    let all_selected = files
        .iter()
        .all(|f| f.selection.selection_type() == DiffSelectionType::All);
    let none_selected = files
        .iter()
        .all(|f| f.selection.selection_type() == DiffSelectionType::None);
    match (all_selected, none_selected) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    }
}
