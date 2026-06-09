use std::path::Path;
use std::process::Command;

use serde::Serialize;

/// Safety summary of a git working copy, used by `banco tidy` to warn the user
/// before a no-longer-synced repository is removed. Every field errs toward
/// caution: when git can't be queried, the repo is reported as not safe.
#[derive(Serialize, Default)]
pub struct RepoState {
    /// Tracked files with staged or unstaged modifications.
    pub uncommitted_changes: bool,
    /// Number of untracked files (excluding ignored).
    pub untracked_files: usize,
    /// Commits on local branches not present on any remote.
    pub unpushed_commits: usize,
    /// Local branches with no upstream — work that may exist only here.
    pub local_only_branches: Vec<String>,
    /// Number of entries in the stash.
    pub stashes: usize,
    /// True only when nothing above would be lost by deleting the directory.
    pub safe_to_remove: bool,
    /// Set when git could not be queried (not a repo, git missing, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn git_lines(repo_path: &Path, args: &[&str]) -> Option<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(
        text.lines()
            .map(str::trim_end)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect(),
    )
}

/// Inspect a git working copy for anything that would be lost on deletion.
/// Returns a populated `RepoState`; on any git failure the `error` field is set
/// and `safe_to_remove` stays false.
pub fn inspect_repo(repo_path: &Path) -> RepoState {
    if !repo_path.join(".git").exists() {
        return RepoState {
            error: Some("not a git repository".to_string()),
            ..Default::default()
        };
    }

    let Some(status) = git_lines(repo_path, &["status", "--porcelain"]) else {
        return RepoState {
            error: Some("git status failed".to_string()),
            ..Default::default()
        };
    };
    let untracked_files = status.iter().filter(|l| l.starts_with("??")).count();
    let uncommitted_changes = status.iter().any(|l| !l.starts_with("??"));

    let unpushed_commits = git_lines(repo_path, &["log", "--branches", "--not", "--remotes", "--oneline"])
        .map(|l| l.len())
        .unwrap_or(0);

    let local_only_branches = git_lines(
        repo_path,
        &["for-each-ref", "--format=%(refname:short) %(upstream)", "refs/heads"],
    )
    .unwrap_or_default()
    .into_iter()
    .filter_map(|line| {
        let mut parts = line.splitn(2, ' ');
        let name = parts.next().unwrap_or("").to_string();
        let upstream = parts.next().unwrap_or("").trim();
        if upstream.is_empty() {
            Some(name)
        } else {
            None
        }
    })
    .collect::<Vec<_>>();

    let stashes = git_lines(repo_path, &["stash", "list"])
        .map(|l| l.len())
        .unwrap_or(0);

    let safe_to_remove = !uncommitted_changes
        && untracked_files == 0
        && unpushed_commits == 0
        && local_only_branches.is_empty()
        && stashes == 0;

    RepoState {
        uncommitted_changes,
        untracked_files,
        unpushed_commits,
        local_only_branches,
        stashes,
        safe_to_remove,
        error: None,
    }
}

pub fn read_git_remote_url(repo_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(repo_path.join(".git/config")).ok()?;
    let mut in_origin = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == r#"[remote "origin"]"# {
            in_origin = true;
        } else if trimmed.starts_with('[') {
            in_origin = false;
        } else if in_origin {
            if let Some(url) = trimmed.strip_prefix("url = ") {
                return Some(url.to_string());
            }
        }
    }
    None
}

pub fn normalize_git_url(url: &str) -> String {
    let url = url.trim_end_matches(".git");
    if let Some(rest) = url.strip_prefix("git@") {
        if let Some((host, path)) = rest.split_once(':') {
            return format!("https://{}/{}", host, path);
        }
    }
    url.to_string()
}
