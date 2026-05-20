//! Native commit creation — translation of app/src/lib/git/commit.ts.
//!
//! The TypeScript version shells out to `git commit -F -`.  This version
//! uses libgit2 exclusively:
//!   1. Reset the index to HEAD  (= `unstageAll`)
//!   2. Add / remove each selected file in the index  (= `stageFiles`)
//!   3. Write the index to an ODB tree
//!   4. Create the commit object natively via `repo.commit()`

use crate::git_engine::GitEngine;
use crate::models::commit::CommitContext;
use crate::models::status::{AppFileStatusKind, DiffSelectionType, WorkingDirectoryFileChange};
use git2::Repository as Git2Repository;

impl GitEngine {
    /// Create a new commit (or amend the current one) from a set of
    /// working-directory file changes.
    ///
    /// # Arguments
    /// * `repo`    – An already-opened libgit2 repository handle.
    /// * `context` – The commit message and amend flag from the UI.
    /// * `files`   – The full list of working-directory changes.  Only files
    ///               whose `selection_type()` is **not** `None` are staged.
    ///
    /// # Returns
    /// The `Oid` of the newly created commit on success.
    pub fn create_commit(
        repo: &Git2Repository,
        context: &CommitContext,
        files: &[WorkingDirectoryFileChange],
    ) -> Result<git2::Oid, git2::Error> {
        // ── Step 1: obtain the default signature (name + email from git config) ──
        let sig = repo.signature()?;

        // ── Step 2: open the index ────────────────────────────────────────────────
        let mut index = repo.index()?;

        // ── Step 3: reset index to HEAD  (= unstageAll) ───────────────────────────
        //
        // Reading the HEAD tree into the in-memory index discards any previously
        // staged changes, giving us a clean slate identical to the TypeScript
        // `unstageAll` call.  On the very first commit HEAD does not exist yet,
        // so we skip this step and start from an empty index.
        let maybe_parent: Option<git2::Commit<'_>> = match repo.head() {
            Ok(head_ref) => {
                let parent_commit = head_ref.peel_to_commit()?;
                let head_tree = parent_commit.tree()?;
                index.read_tree(&head_tree)?;
                Some(parent_commit)
            }
            Err(_) => {
                // No HEAD yet — this is the initial commit.
                None
            }
        };

        // ── Step 4: stage selected files  (= stageFiles) ─────────────────────────
        //
        // Files with `DiffSelectionType::None` are intentionally excluded from the
        // commit, so we skip them.  All other files (All / Partial) are staged.
        // Deleted files must be removed from the index rather than added.
        for wdf in files {
            if wdf.selection.selection_type() == DiffSelectionType::None {
                continue;
            }

            let path = &wdf.file_change.path;
            let is_deleted = wdf.file_change.status.kind() == AppFileStatusKind::Deleted;

            if is_deleted {
                // `remove_path` is best-effort; the file may not be in the index
                // at all if it was already untracked — that is fine.
                let _ = index.remove_path(path);
            } else {
                // `add_path` resolves the path relative to the workdir and adds
                // (or replaces) the entry in the in-memory index.
                index.add_path(path)?;
            }
        }

        // ── Step 5: write the in-memory index back to disk, then to the ODB ──────
        //
        // `index.write()` persists the index to `.git/index` so that the working
        // tree appears clean after the commit.
        // `index.write_tree()` serialises the index as a tree object and returns
        // its Oid — this is the tree the commit will point at.
        index.write()?;
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;

        // ── Step 6: build the commit message ─────────────────────────────────────
        let message = match &context.description {
            Some(desc) if !desc.trim().is_empty() => {
                format!("{}\n\n{}", context.summary.trim(), desc.trim_end())
            }
            _ => context.summary.trim().to_string(),
        };

        // ── Step 7: create or amend the commit ───────────────────────────────────
        if context.amend {
            // Amend replaces the current HEAD commit in-place, reusing its
            // parents so the graph topology is preserved.
            let head_commit = repo
                .head()?
                .peel_to_commit()
                .map_err(|e| git2::Error::from_str(&format!("amend requires HEAD: {e}")))?;

            head_commit.amend(
                Some("HEAD"), // update HEAD ref
                Some(&sig),   // new author
                Some(&sig),   // new committer
                None,         // keep encoding
                Some(&message),
                Some(&tree),
            )
        } else {
            // Normal commit — collect parents into a slice that `repo.commit`
            // can borrow.  On the initial commit `maybe_parent` is `None` and
            // we pass an empty slice, which is exactly what libgit2 expects.
            let parent_refs: Vec<&git2::Commit<'_>> = maybe_parent.iter().collect();

            repo.commit(
                Some("HEAD"), // update HEAD ref
                &sig,         // author
                &sig,         // committer
                &message,
                &tree,
                &parent_refs,
            )
        }
    }
}
