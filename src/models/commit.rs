//! Translations of:
//!   commit-identity.ts  →  CommitIdentity
//!   git-author.ts       →  GitAuthor
//!   commit.ts           →  CommitOneLine, CommitContext, Commit

use chrono::{DateTime, FixedOffset};

// ---------------------------------------------------------------------------
// CommitIdentity  (commit-identity.ts)
// ---------------------------------------------------------------------------

/// A name, email, and timestamp for the author or committer of a commit.
///
/// Maps to the TypeScript `CommitIdentity` class.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitIdentity {
    pub name: String,
    pub email: String,
    /// The exact moment the action occurred, including the author's local
    /// timezone offset (preserves the `+0200` / `-0700` part of git output).
    pub date: DateTime<FixedOffset>,
}

impl CommitIdentity {
    pub fn new(
        name: impl Into<String>,
        email: impl Into<String>,
        date: DateTime<FixedOffset>,
    ) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            date,
        }
    }

    /// Returns the timezone offset in whole minutes from UTC, matching the TS
    /// `tzOffset` property (positive = east of UTC).
    pub fn tz_offset_minutes(&self) -> i32 {
        // chrono's `local_minus_utc` is in seconds; convert to minutes.
        self.date.offset().local_minus_utc() / 60
    }
}

impl std::fmt::Display for CommitIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}

// ---------------------------------------------------------------------------
// GitAuthor  (git-author.ts)
// ---------------------------------------------------------------------------

/// A parsed co-author, typically extracted from a `Co-Authored-By:` trailer.
///
/// Maps to the TypeScript `GitAuthor` class.
#[derive(Debug, Clone, PartialEq)]
pub struct GitAuthor {
    pub name: String,
    pub email: String,
}

impl GitAuthor {
    /// Parse a `"Name <email>"` string, returning `None` if the format is invalid.
    pub fn parse(name_addr: &str) -> Option<Self> {
        let lt = name_addr.find('<')?;
        let gt = name_addr.find('>')?;
        if gt < lt {
            return None;
        }
        Some(Self {
            name: name_addr[..lt].trim().to_string(),
            email: name_addr[lt + 1..gt].trim().to_string(),
        })
    }
}

impl std::fmt::Display for GitAuthor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}

// ---------------------------------------------------------------------------
// Commit types  (commit.ts)
// ---------------------------------------------------------------------------

/// The minimal data needed to represent a commit (equivalent to
/// `git log --oneline --no-abbrev-commit`).
///
/// Maps to the TypeScript `CommitOneLine` type.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitOneLine {
    /// The full 40-character commit SHA.
    pub sha: String,
    /// The first line of the commit message.
    pub summary: String,
}

/// Everything required to *create* a new commit.
///
/// Maps to the TypeScript `ICommitContext` interface.
#[derive(Debug, Clone)]
pub struct CommitContext {
    /// The commit subject / first line (required).
    pub summary: String,
    /// The optional commit body (everything after the subject line).
    pub description: Option<String>,
    /// Whether to amend the most recent commit instead of creating a new one.
    pub amend: bool,
    /// Zero or more co-authors to append as `Co-Authored-By:` trailers.
    pub co_authors: Vec<GitAuthor>,
}

impl Default for CommitContext {
    fn default() -> Self {
        Self {
            summary: String::new(),
            description: None,
            amend: false,
            co_authors: Vec::new(),
        }
    }
}

/// A fully loaded git commit.
///
/// Maps to the TypeScript `Commit` class.
#[derive(Debug, Clone)]
pub struct Commit {
    /// The full 40-character SHA.
    pub sha: String,
    /// The abbreviated SHA (first 9 characters by convention).
    pub short_sha: String,
    /// The subject line of the commit message.
    pub summary: String,
    /// Everything after the subject line, with `Co-Authored-By` trailers removed.
    pub body: String,
    pub author: CommitIdentity,
    pub committer: CommitIdentity,
    /// SHAs of parent commits. More than one means this is a merge commit.
    pub parent_shas: Vec<String>,
    /// Tags pointing at this commit.
    pub tags: Vec<String>,
    /// Co-authors parsed from `Co-Authored-By:` trailers.
    pub co_authors: Vec<GitAuthor>,
}

impl Commit {
    /// Returns `true` when the author and committer are the same person.
    pub fn authored_by_committer(&self) -> bool {
        self.author.name == self.committer.name && self.author.email == self.committer.email
    }

    /// Returns `true` when the commit has more than one parent (i.e. is a merge).
    pub fn is_merge_commit(&self) -> bool {
        self.parent_shas.len() > 1
    }

    /// Returns the first 9 characters of a SHA, matching `shortenSHA` in TS.
    pub fn shorten_sha(sha: &str) -> &str {
        &sha[..sha.len().min(9)]
    }
}
