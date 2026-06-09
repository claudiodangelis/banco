//! Detection for `banco tidy`: finds module data on disk that is no longer
//! backed by the current configuration — repositories dropped from a provider,
//! task trees whose syncing was turned off, and local items a user may want to
//! retire. Detection only: this module never deletes anything. It emits a
//! structured report that the `tidy` skill uses to brief the user, who always
//! has the last word on removal.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::Serialize;

use crate::config;
use crate::providers::frontmatter;
use crate::providers::git::{self, normalize_git_url, read_git_remote_url, RepoState};

#[derive(Serialize)]
pub struct TidyReport {
    pub repos: Vec<RepoFinding>,
    pub tasks: Vec<TaskFinding>,
    pub local: Vec<LocalFinding>,
}

/// A synced repository directory that no longer corresponds to the config.
#[derive(Serialize)]
pub struct RepoFinding {
    pub provider: String,
    pub name: String,
    /// Path relative to the project root.
    pub path: String,
    pub reason: &'static str,
    /// Git safety summary — what would be lost by removing the directory.
    pub git: RepoState,
}

/// A task directory whose issues are no longer synced.
#[derive(Serialize)]
pub struct TaskFinding {
    pub provider: String,
    /// Path relative to the project root.
    pub path: String,
    pub reason: &'static str,
    pub files: usize,
    pub open: usize,
    pub closed: usize,
}

/// A local item surfaced for review before the user retires a module.
#[derive(Serialize)]
pub struct LocalFinding {
    pub module: String,
    /// Path relative to the project root.
    pub path: String,
    /// First line looks like a URL (bookmarks store the URL on line one).
    pub has_url: bool,
    pub body_lines: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

#[derive(PartialEq)]
enum Kind {
    GitHub,
    GitLab,
    Other,
}

/// Everything tidy needs to know about a configured provider, derived from
/// config alone (no network). Disabled providers are included — a disabled
/// provider is itself a reason data is stale.
struct ProviderInfo {
    kind: Kind,
    enabled: bool,
    /// Whether issue syncing is on (github/gitlab `sync_issues`, default true).
    syncs_tasks: bool,
    /// Explicit `projects` as full paths (owner/repo, namespace/project).
    explicit: Option<HashSet<String>>,
    /// Compiled `projects_pattern`, if set.
    pattern: Option<Regex>,
}

impl ProviderInfo {
    /// Does this provider still expect a project at the given full path
    /// (`owner/repo` or `namespace/project`)? `None` means "can't tell" — the
    /// provider has neither list nor pattern, or the path couldn't be
    /// reconstructed — and the caller should not flag the item as stale.
    fn expects(&self, full_path: Option<&str>) -> Option<bool> {
        match (&self.explicit, &self.pattern) {
            (Some(set), _) => match full_path {
                Some(p) => Some(set.contains(p)),
                // No reconstructed path to compare against an explicit list.
                None => None,
            },
            (None, Some(re)) => full_path.map(|p| re.is_match(p)),
            (None, None) => None,
        }
    }
}

fn build_provider_map(cfg: &config::ProjectConfig) -> HashMap<String, ProviderInfo> {
    let mut map = HashMap::new();
    for entry in &cfg.providers {
        let kind = match entry.name.as_str() {
            "github" => Kind::GitHub,
            "gitlab" => Kind::GitLab,
            _ => Kind::Other,
        };
        let explicit = {
            let list = entry.get_list("projects");
            if list.is_empty() {
                None
            } else {
                Some(list.into_iter().collect::<HashSet<_>>())
            }
        };
        let pattern = entry
            .get_str("projects_pattern")
            .and_then(|p| Regex::new(&format!("^(?:{})$", p)).ok());
        map.insert(
            entry.display_name().to_string(),
            ProviderInfo {
                kind,
                enabled: entry.enabled,
                syncs_tasks: entry.get_bool("sync_issues", true),
                explicit,
                pattern,
            },
        );
    }
    map
}

/// Reconstruct the provider-side full path (`owner/repo`, `namespace/project`)
/// from a cloned repo's git remote, so it can be tested against config offline.
fn repo_full_path(repo_path: &Path) -> Option<String> {
    let url = read_git_remote_url(repo_path)?;
    let base = normalize_git_url(&url); // https://host/owner/repo[/...]
    base.splitn(4, '/').nth(3).map(String::from)
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    out
}

/// Count `.md` task files under `dir` (recursively) and split by frontmatter
/// status so the skill can warn "you still have N open issues here".
fn count_tasks(dir: &Path) -> (usize, usize, usize) {
    let (mut files, mut open, mut closed) = (0, 0, 0);
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            files += 1;
            if let Ok(content) = std::fs::read_to_string(path) {
                match frontmatter::parse(&content).0.map(|f| f.status) {
                    Some(s) if s == "closed" => closed += 1,
                    Some(_) => open += 1,
                    None => {}
                }
            }
        }
    }
    (files, open, closed)
}

fn detect_repos(root: &Path, providers: &HashMap<String, ProviderInfo>) -> Vec<RepoFinding> {
    let repos_root = root.join("repos");
    let mut findings = Vec::new();

    for provider_dir in subdirs(&repos_root) {
        let display = provider_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if display == "local" {
            continue;
        }

        let info = providers.get(&display);

        for repo_dir in subdirs(&provider_dir) {
            let name = repo_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let reason: Option<&'static str> = match info {
                None => Some("provider_removed"),
                Some(p) if !p.enabled => Some("provider_disabled"),
                Some(p) if p.kind == Kind::Other => None,
                Some(p) => {
                    let full = repo_full_path(&repo_dir);
                    match p.expects(full.as_deref()) {
                        Some(false) if p.pattern.is_some() => Some("no_longer_matches_pattern"),
                        Some(false) => Some("removed_from_config"),
                        _ => None,
                    }
                }
            };

            if let Some(reason) = reason {
                findings.push(RepoFinding {
                    provider: display.clone(),
                    name,
                    path: rel(root, &repo_dir),
                    reason,
                    git: git::inspect_repo(&repo_dir),
                });
            }
        }
    }

    findings
}

fn detect_tasks(root: &Path, providers: &HashMap<String, ProviderInfo>) -> Vec<TaskFinding> {
    let tasks_root = root.join("tasks");
    let mut findings = Vec::new();

    for provider_dir in subdirs(&tasks_root) {
        let display = provider_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if display == "local" {
            continue;
        }

        let info = providers.get(&display);

        // Provider-level reasons cover the whole tree in one finding.
        let provider_reason: Option<&'static str> = match info {
            None => Some("provider_removed"),
            Some(p) if !p.enabled => Some("provider_disabled"),
            Some(p) if !p.syncs_tasks => Some("sync_disabled"),
            Some(_) => None,
        };

        if let Some(reason) = provider_reason {
            let (files, open, closed) = count_tasks(&provider_dir);
            if files > 0 {
                findings.push(TaskFinding {
                    provider: display.clone(),
                    path: rel(root, &provider_dir),
                    reason,
                    files,
                    open,
                    closed,
                });
            }
            continue;
        }

        // Provider still syncs: flag only the project subtrees dropped from
        // config. GitHub task paths are owner/repo (the full project path is
        // recoverable from the layout); GitLab paths are a single project slug,
        // so per-project detection only applies to explicit `projects` lists.
        let Some(info) = info else { continue };
        match info.kind {
            Kind::GitHub => {
                for owner_dir in subdirs(&provider_dir) {
                    let owner = owner_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    for repo_dir in subdirs(&owner_dir) {
                        let repo = repo_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        let full = format!("{}/{}", owner, repo);
                        if let Some(reason) = stale_project_reason(info, &full) {
                            push_task_finding(root, &display, &repo_dir, reason, &mut findings);
                        }
                    }
                }
            }
            Kind::GitLab => {
                // Only explicit lists give us the full namespace/project to
                // match a slug against; skip pattern users to avoid false hits.
                if let Some(set) = &info.explicit {
                    let slugs: HashSet<&str> = set
                        .iter()
                        .map(|p| p.rsplit('/').next().unwrap_or(p))
                        .collect();
                    for project_dir in subdirs(&provider_dir) {
                        let slug = project_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        if !slugs.contains(slug) {
                            push_task_finding(
                                root,
                                &display,
                                &project_dir,
                                "removed_from_config",
                                &mut findings,
                            );
                        }
                    }
                }
            }
            Kind::Other => {}
        }
    }

    findings
}

fn stale_project_reason(info: &ProviderInfo, full_path: &str) -> Option<&'static str> {
    match info.expects(Some(full_path)) {
        Some(false) if info.pattern.is_some() => Some("no_longer_matches_pattern"),
        Some(false) => Some("removed_from_config"),
        _ => None,
    }
}

fn push_task_finding(
    root: &Path,
    provider: &str,
    dir: &Path,
    reason: &'static str,
    findings: &mut Vec<TaskFinding>,
) {
    let (files, open, closed) = count_tasks(dir);
    if files > 0 {
        findings.push(TaskFinding {
            provider: provider.to_string(),
            path: rel(root, dir),
            reason,
            files,
            open,
            closed,
        });
    }
}

/// Surface local items (notes/bookmarks) for review before a user retires a
/// module. Unlike providers, local modules have no config flag, so this only
/// runs when the user explicitly asks for a module via `--module`.
fn detect_local(root: &Path, module: &str) -> anyhow::Result<Vec<LocalFinding>> {
    let base = root.join(module).join("local");
    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut findings = Vec::new();
    for entry in walkdir::WalkDir::new(&base)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !(path.is_file() && path.extension().map_or(false, |e| e == "md")) {
            continue;
        }
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let first = content.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        let has_url = first.trim_start().starts_with("http://")
            || first.trim_start().starts_with("https://");
        let body_lines = content.lines().filter(|l| !l.trim().is_empty()).count();
        let modified = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .map(|t| DateTime::<Utc>::from(t).format("%Y-%m-%d").to_string());

        findings.push(LocalFinding {
            module: module.to_string(),
            path: rel(root, path),
            has_url,
            body_lines,
            modified,
        });
    }
    findings.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(findings)
}

/// Build the full stale-data report. When `module` is set, only that module is
/// scanned (and local modules are included only via this path).
pub fn report(root: &Path, module: Option<&str>) -> anyhow::Result<TidyReport> {
    let cfg = config::load(root).context("failed to load config")?;
    let providers = build_provider_map(&cfg);

    let scan = |m: &str| module.map_or(true, |only| only == m);

    let repos = if scan("repos") {
        detect_repos(root, &providers)
    } else {
        Vec::new()
    };
    let tasks = if scan("tasks") {
        detect_tasks(root, &providers)
    } else {
        Vec::new()
    };

    // Local modules are reviewed only on explicit request.
    let local = match module {
        Some(m @ ("notes" | "bookmarks")) => detect_local(root, m)?,
        _ => Vec::new(),
    };

    Ok(TidyReport { repos, tasks, local })
}
