//! Translation of app/src/lib/git/checkout.ts → native libgit2.
//!
//! The TypeScript implementation shells out to `git checkout --progress …`.
//! This version uses libgit2's `Repository::set_head` + `Repository::checkout_head`
//! so there are zero child-process spawns.

use crate::git_engine::GitEngine;
use git2::{BranchType as Git2BranchType, Repository as Git2Repository, build::CheckoutBuilder};

impl GitEngine {
    /// Check out a local branch by name.
    ///
    /// Steps:
    ///  1. Resolve the branch to its canonical `refs/heads/<name>` ref.
    ///  2. Call `repo.set_head()` to move HEAD.
    ///  3. Call `repo.checkout_head()` with a *safe* `CheckoutBuilder` to
    ///     update the working tree and index natively.
    pub fn checkout_branch(repo: &Git2Repository, branch_name: &str) -> Result<(), git2::Error> {
        // ── Step 1: resolve the branch ────────────────────────────────────────
        let branch = repo.find_branch(branch_name, Git2BranchType::Local)?;

        // FIX: The compiler explicitly states this is a Result<&str, git2::Error>.
        // We remove the inner `Some()` and just match the `Ok(name)`.
        let ref_name = match branch.get().name() {
            Ok(name) => name,
            Err(e) => return Err(e),
        };

        // ── Step 2: move HEAD ─────────────────────────────────────────────────
        // Atomically writes `ref: refs/heads/<name>` into `.git/HEAD`.
        repo.set_head(ref_name)?;

        // ── Step 3: update the working tree ───────────────────────────────────
        // `CheckoutBuilder::safe()` updates tracked files but refuses to overwrite
        // any file that has uncommitted local modifications.
        let mut builder = CheckoutBuilder::new();
        builder.safe();

        repo.checkout_head(Some(&mut builder))?;

        Ok(())
    }
}
