use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use serde::Serialize;

/// Default number of concurrent git clone/fetch operations. Repo syncing is
/// network-bound (transfer + remote server), not CPU-bound, so this is a small
/// fixed value rather than something derived from core count: going wider mainly
/// risks tripping remote connection/rate limits and splitting the same pipe.
pub const DEFAULT_SYNC_JOBS: usize = 6;

enum Outcome {
    Cloned,
    Fetched,
    /// `git fetch` failed for an existing repo — non-fatal, reported as a warning.
    FetchWarning(String),
    /// Resolve or clone failed — fatal; the overall sync returns an error.
    Failed(String),
}

/// Clone (or fetch, if already present) a set of repositories in parallel.
///
/// `resolve` maps a project identifier to its `(ssh_url, destination)`; it runs
/// inside the worker threads so per-repo API lookups are parallelized too. Output
/// from the underlying git processes is captured and discarded — only a final
/// summary is printed. Returns an error if any repo failed to resolve or clone;
/// fetch failures on existing repos are reported as warnings but not fatal.
pub fn sync_repos<F>(
    projects: &[String],
    repos_root: &Path,
    concurrency: usize,
    resolve: F,
) -> anyhow::Result<()>
where
    F: Fn(&str) -> anyhow::Result<(String, PathBuf)> + Sync,
{
    if projects.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(repos_root)?;

    let workers = concurrency.clamp(1, projects.len());
    let next = AtomicUsize::new(0);
    let outcomes: Mutex<Vec<Outcome>> = Mutex::new(Vec::with_capacity(projects.len()));

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= projects.len() {
                    break;
                }
                let label = &projects[i];
                let outcome = match resolve(label) {
                    Ok((ssh_url, dest)) => clone_or_fetch(label, &ssh_url, &dest),
                    Err(e) => Outcome::Failed(format!("{}: {}", label, e)),
                };
                outcomes.lock().unwrap().push(outcome);
            });
        }
    });

    let outcomes = outcomes.into_inner().unwrap();
    let mut cloned = 0usize;
    let mut fetched = 0usize;
    let mut warnings = Vec::new();
    let mut failures = Vec::new();
    for outcome in outcomes {
        match outcome {
            Outcome::Cloned => cloned += 1,
            Outcome::Fetched => fetched += 1,
            Outcome::FetchWarning(m) => warnings.push(m),
            Outcome::Failed(m) => failures.push(m),
        }
    }

    println!("  repos: {} cloned, {} updated", cloned, fetched);
    for w in &warnings {
        eprintln!("  warning: git fetch failed for {}", w);
    }
    if !failures.is_empty() {
        anyhow::bail!("git clone failed for {} repo(s):\n  {}", failures.len(), failures.join("\n  "));
    }
    Ok(())
}

fn clone_or_fetch(label: &str, ssh_url: &str, dest: &Path) -> Outcome {
    if dest.exists() {
        match Command::new("git")
            .args(["-C", dest.to_str().unwrap(), "fetch", "--all", "--prune"])
            .output()
        {
            Ok(o) if o.status.success() => Outcome::Fetched,
            _ => Outcome::FetchWarning(label.to_string()),
        }
    } else {
        match Command::new("git")
            .args(["clone", ssh_url, dest.to_str().unwrap()])
            .output()
        {
            Ok(o) if o.status.success() => Outcome::Cloned,
            Ok(o) => Outcome::Failed(format!(
                "{}: {}",
                label,
                String::from_utf8_lossy(&o.stderr).trim()
            )),
            Err(e) => Outcome::Failed(format!("{}: {}", label, e)),
        }
    }
}

/// Safety summary of a git working copy, used by `banco tidy` to warn the user
/// before a no-longer-synced repository is removed. Every field errs toward
/// caution: when git can't be queried, the repo is reported as not safe.
#[derive(Serialize, Default)]
pub struct RepoState {
    /// Current branch, or `None` for detached HEAD / unqueryable repo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Tracked files with staged or unstaged modifications.
    pub uncommitted_changes: bool,
    /// Number of untracked files (excluding ignored).
    pub untracked_files: usize,
    /// Local branches not merged into the current branch.
    pub unmerged_branches: usize,
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

    let unmerged_branches = git_lines(repo_path, &["branch", "--no-merged"])
        .map(|l| l.len())
        .unwrap_or(0);

    let safe_to_remove = !uncommitted_changes
        && untracked_files == 0
        && unpushed_commits == 0
        && local_only_branches.is_empty()
        && stashes == 0;

    RepoState {
        branch: current_branch(repo_path),
        uncommitted_changes,
        untracked_files,
        unmerged_branches,
        unpushed_commits,
        local_only_branches,
        stashes,
        safe_to_remove,
        error: None,
    }
}

/// Current branch name of a git working copy, or `None` when the path is not a
/// git repo, git can't be queried, or HEAD is detached.
pub fn current_branch(repo_path: &Path) -> Option<String> {
    let branch = git_lines(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])?
        .pop()?;
    if branch == "HEAD" {
        None
    } else {
        Some(branch)
    }
}

/// Lightweight branch/dirtiness summary used to annotate repos in the dashboard.
pub struct RepoStatus {
    /// Current branch, or `None` for detached HEAD / unqueryable repo.
    pub branch: Option<String>,
    /// True when the working copy has staged, unstaged, or untracked changes.
    pub dirty: bool,
    /// Local branches not merged into the current branch.
    pub unmerged_branches: usize,
}

/// Build a repos-module context item annotated with the working copy's branch,
/// dirtiness, and unmerged-branch count. Shared by every provider's repos module.
pub fn repo_item(name: String, repo_path: &Path) -> serde_json::Value {
    let status = repo_status(repo_path);
    serde_json::json!({
        "name": name,
        "branch": status.as_ref().and_then(|s| s.branch.clone()),
        "dirty": status.as_ref().map(|s| s.dirty).unwrap_or(false),
        "unmerged_branches": status.as_ref().map(|s| s.unmerged_branches).unwrap_or(0),
    })
}

/// Summarize a git working copy for display. Returns `None` when the path is not
/// a git repository; individual git failures degrade gracefully to defaults.
pub fn repo_status(repo_path: &Path) -> Option<RepoStatus> {
    if !repo_path.join(".git").exists() {
        return None;
    }
    let dirty = git_lines(repo_path, &["status", "--porcelain"])
        .map(|l| !l.is_empty())
        .unwrap_or(false);
    let unmerged_branches = git_lines(repo_path, &["branch", "--no-merged"])
        .map(|l| l.len())
        .unwrap_or(0);
    Some(RepoStatus {
        branch: current_branch(repo_path),
        dirty,
        unmerged_branches,
    })
}

/// A single commit from `git log`, for the read-only history view.
pub struct Commit {
    pub short_hash: String,
    pub author: String,
    /// Author date, ISO-8601 / strict (e.g. `2026-07-10 14:22:01 +0200`).
    pub date: String,
    pub subject: String,
}

/// Return up to `max` most-recent commits reachable from HEAD. Returns `None`
/// when the path is not a git repository or git can't be queried (e.g. an empty
/// repo with no commits yet). Fields are split on NUL to stay robust against any
/// character inside a subject or author name.
pub fn git_log(repo_path: &Path, max: usize) -> Option<Vec<Commit>> {
    if !repo_path.join(".git").exists() {
        return None;
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args([
            "log",
            &format!("--max-count={max}"),
            "--no-color",
            "--pretty=format:%h%x00%an%x00%ad%x00%s%x1e",
            "--date=format:%Y-%m-%d %H:%M:%S %z",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let commits = text
        .split('\u{1e}')
        .map(str::trim_start)
        .filter(|r| !r.is_empty())
        .filter_map(|record| {
            let mut f = record.split('\u{0}');
            Some(Commit {
                short_hash: f.next()?.to_string(),
                author: f.next()?.to_string(),
                date: f.next()?.to_string(),
                subject: f.next().unwrap_or("").to_string(),
            })
        })
        .collect();
    Some(commits)
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
