use crate::git_engine::GitEngine;
use crate::git_engine::history::git2_time_to_datetime;
use crate::models::branch::{Branch, BranchTip, BranchType};
use crate::models::commit::CommitIdentity;
use git2::{BranchType as Git2BranchType, Repository as Git2Repository};

impl GitEngine {
    pub fn get_branches(repo: &Git2Repository) -> Result<Vec<Branch>, git2::Error> {
        let branch_iter = repo.branches(None)?;
        let mut branches: Vec<Branch> = Vec::new();

        for item in branch_iter {
            let (git2_branch, git2_type) = match item {
                Ok(pair) => pair,
                Err(_) => continue,
            };

            let branch_type = match git2_type {
                Git2BranchType::Local => BranchType::Local,
                Git2BranchType::Remote => BranchType::Remote,
            };

            // FIX 1: Explicitly match both the Result and the inner Option safely
            let name: String = match git2_branch.name() {
                Ok(Some(n)) => n.to_owned(),
                _ => continue, // Skip branches with invalid UTF-8 names
            };

            let git_ref: String = match git2_branch.name() {
                Ok(Some(r)) => r.to_owned(),
                _ => continue, // Skip if the underlying ref name is missing or invalid UTF-8
            };

            let upstream: Option<String> = if branch_type == BranchType::Local {
                resolve_upstream_name(&git2_branch)
            } else {
                None
            };

            let tip_commit = match git2_branch.get().peel_to_commit() {
                Ok(c) => c,
                Err(_) => continue,
            };

            let tip = build_branch_tip(&tip_commit);

            branches.push(Branch {
                name,
                upstream,
                tip,
                branch_type,
                git_ref,
            });
        }

        branches.sort_by(|a, b| {
            a.branch_type
                .cmp(&b.branch_type)
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok(branches)
    }
}

// FIX 2: Rewritten with explicit matching to perfectly align with Option<String>
fn resolve_upstream_name(branch: &git2::Branch<'_>) -> Option<String> {
    // branch.upstream() returns Result<Branch, git2::Error>
    let upstream_branch = match branch.upstream() {
        Ok(ub) => ub,
        Err(_) => return None, // No upstream configured
    };

    // upstream_branch.name() returns Result<Option<&str>, git2::Error>
    // Note: in modern git2 versions, this automatically strips "refs/remotes/"
    match upstream_branch.name() {
        Ok(Some(n)) => Some(n.to_owned()),
        _ => None,
    }
}

fn build_branch_tip(commit: &git2::Commit<'_>) -> BranchTip {
    let sha = commit.id().to_string();
    let git2_author = commit.author();
    let author = CommitIdentity::new(
        git2_author.name().unwrap_or(""),
        git2_author.email().unwrap_or(""),
        git2_time_to_datetime(git2_author.when()),
    );
    BranchTip { sha, author }
}
