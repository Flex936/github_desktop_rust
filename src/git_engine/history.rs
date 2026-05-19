use crate::git_engine::GitEngine;
use crate::models::commit::{Commit, CommitIdentity};
use chrono::{DateTime, FixedOffset, TimeZone};
use git2::{Repository as Git2Repository, Sort};

impl GitEngine {
    pub fn get_commit_history(
        repo: &Git2Repository,
        limit: usize,
    ) -> Result<Vec<Commit>, git2::Error> {
        let mut walk = repo.revwalk()?;
        walk.push_head()?;
        walk.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)?;

        let mut commits: Vec<Commit> = Vec::with_capacity(limit.min(256));

        for oid_result in walk.take(limit) {
            let oid = oid_result?;
            let git2_commit = repo.find_commit(oid)?;

            let sha = oid.to_string();
            let short_sha = sha[..sha.len().min(9)].to_string();

            let raw_message = git2_commit.message().unwrap_or("").trim_end().to_string();
            let (summary, body) = split_message(&raw_message);

            let git2_author = git2_commit.author();
            let author = CommitIdentity::new(
                git2_author.name().unwrap_or(""),
                git2_author.email().unwrap_or(""),
                git2_time_to_datetime(git2_author.when()),
            );

            let git2_committer = git2_commit.committer();
            let committer = CommitIdentity::new(
                git2_committer.name().unwrap_or(""),
                git2_committer.email().unwrap_or(""),
                git2_time_to_datetime(git2_committer.when()),
            );

            let parent_shas: Vec<String> =
                git2_commit.parent_ids().map(|id| id.to_string()).collect();

            commits.push(Commit {
                sha,
                short_sha,
                summary,
                body,
                author,
                committer,
                parent_shas,
                tags: Vec::new(),
                co_authors: Vec::new(),
            });
        }

        Ok(commits)
    }
}

pub(crate) fn git2_time_to_datetime(time: git2::Time) -> DateTime<FixedOffset> {
    let offset_secs = time.offset_minutes() * 60;
    let zone = FixedOffset::east_opt(offset_secs)
        .unwrap_or_else(|| FixedOffset::east_opt(0).expect("UTC is always valid"));

    zone.timestamp_opt(time.seconds(), 0)
        .single()
        .unwrap_or_else(|| {
            FixedOffset::east_opt(0)
                .expect("UTC is always valid")
                .timestamp_opt(0, 0)
                .single()
                .expect("Unix epoch is always valid")
        })
}

fn split_message(raw: &str) -> (String, String) {
    let mut lines = raw.splitn(2, "\n\n");
    let summary = lines.next().unwrap_or("").trim().to_string();
    let body = lines.next().unwrap_or("").trim_end().to_string();
    (summary, body)
}
