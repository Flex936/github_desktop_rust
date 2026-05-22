//! Native diff generation — translation of app/src/lib/git/diff.ts.
//!
//! Strategy (mirrors GitHub Desktop's own priority):
//!   1. Index vs. Working Directory  (unstaged changes)
//!   2. HEAD vs. Working Directory   (fallback — captures staged-only changes
//!      that have no additional working-tree delta)

use crate::git_engine::GitEngine;
use git2::{DiffOptions, Repository as Git2Repository};
use std::path::Path;

impl GitEngine {
    /// Return the unified diff patch for a single file.
    ///
    /// The returned string is a standard unified-diff patch, suitable for
    /// line-by-line rendering in the UI (headers, hunks, `+`/`-`/` ` lines).
    pub fn get_file_diff(
        repo: &Git2Repository,
        path: &Path,
    ) -> Result<String, git2::Error> {
        // Build options that restrict the diff to the one requested file.
        let mut opts = DiffOptions::new();
        opts.pathspec(path.to_string_lossy().as_ref());
        opts.context_lines(3);

        // ── Pass 1: Index vs. Working Directory ──────────────────────────────
        //
        // This captures files that are *not yet staged* (or partially staged).
        // Passing `None` for the index argument tells libgit2 to read the
        // repository's own on-disk index.
        let index_diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;
        let has_index_changes = index_diff.stats()?.files_changed() > 0;

        let diff = if has_index_changes {
            index_diff
        } else {
            // ── Pass 2: HEAD tree vs. Working Directory ───────────────────────
            //
            // `diff_tree_to_workdir_with_index` includes staged changes that
            // have already been added to the index, so a fully-staged new file
            // still shows up correctly.
            match repo.head() {
                Ok(head_ref) => {
                    let tree = head_ref.peel_to_tree()?;
                    repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))?
                }
                // No HEAD yet (initial commit with everything staged).
                // Return the (empty) index diff — the UI handles the empty case.
                Err(_) => index_diff,
            }
        };

        // ── Serialise the diff to a patch string ─────────────────────────────
        //
        // `DiffFormat::Patch` produces a standard unified diff.
        // The `origin()` character tells us what kind of line we are on:
        //   '+' / '-' / ' '  → added / deleted / context  (need the prefix)
        //   'F'               → file header                (content already complete)
        //   'H'               → hunk header                (content already complete)
        //   'B'               → binary marker              (content already complete)
        let mut patch = String::new();

        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            match line.origin() {
                '+' | '-' | ' ' => patch.push(line.origin()),
                _ => {} // file / hunk headers already contain their own prefix
            }
            if let Ok(s) = std::str::from_utf8(line.content()) {
                patch.push_str(s);
            }
            true
        })?;

        if patch.trim().is_empty() {
            patch = "(No diff — file may be binary, or fully clean.)".to_string();
        }

        Ok(patch)
    }
}