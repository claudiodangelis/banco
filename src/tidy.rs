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
    /// Whole module directories (`repos/`, `tasks/`, …) no enabled provider
    /// backs anymore — see [`ModuleFinding`].
    pub modules: Vec<ModuleFinding>,
}

/// A whole module directory (e.g. `repos/`) that no enabled provider backs: the
/// module is off for the local provider and no remote provider implements it
/// with the module enabled. The per-subtree findings in `repos`/`tasks` still
/// detail what's inside; this is the headline that the whole tree is stale.
#[derive(Serialize)]
pub struct ModuleFinding {
    pub module: String,
    /// Path relative to the project root (the module dir itself, e.g. `repos`).
    pub path: String,
    pub reason: &'static str,
    /// Immediate entries under the module dir (provider subdirs, label dirs, or
    /// files) — a cheap headline count of how much is there.
    pub entries: usize,
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
    /// Whether the tasks module is on (absent from `disabled_modules`).
    tasks_enabled: bool,
    /// Whether the repos module is on (absent from `disabled_modules`).
    repos_enabled: bool,
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

/// Whether the local provider has a given module turned off — either the whole
/// `local` entry is disabled, or the module is in its `disabled_modules`. With
/// no `local` entry at all, the provider runs with defaults (everything on).
fn local_module_disabled(cfg: &config::ProjectConfig, module: &str) -> bool {
    match cfg.providers.iter().find(|e| e.name == "local") {
        Some(e) => !e.enabled || !e.is_module_enabled(module),
        None => false,
    }
}

/// Whether any *enabled* provider in the config still backs `module` with that
/// module enabled. Drives the whole-module highlight: when this is false and the
/// `module/` dir exists, nothing in the config produces that module anymore.
fn module_has_backing(cfg: &config::ProjectConfig, module: &str) -> bool {
    cfg.providers.iter().any(|e| {
        e.enabled
            && e.is_module_enabled(module)
            && provider_implements(&e.name, module)
    })
}

/// Does a provider of this name implement the given module at all? Local backs
/// every module; remote providers back only tasks and repos.
fn provider_implements(name: &str, module: &str) -> bool {
    match name {
        "local" => matches!(module, "notes" | "tasks" | "bookmarks" | "repos"),
        "github" | "gitlab" => matches!(module, "tasks" | "repos"),
        "jira" => module == "tasks",
        _ => false,
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
                tasks_enabled: entry.is_module_enabled("tasks"),
                repos_enabled: entry.is_module_enabled("repos"),
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

fn detect_repos(
    root: &Path,
    providers: &HashMap<String, ProviderInfo>,
    local_repos_disabled: bool,
) -> Vec<RepoFinding> {
    let repos_root = root.join("repos");
    let mut findings = Vec::new();

    for provider_dir in subdirs(&repos_root) {
        let display = provider_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if display == "local" {
            // Local repos are flagged only when the module is turned off for the
            // local provider; otherwise they are user-managed and never stale.
            if local_repos_disabled {
                for repo_dir in subdirs(&provider_dir) {
                    let name = repo_dir.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                    findings.push(RepoFinding {
                        provider: "local".to_string(),
                        name,
                        path: rel(root, &repo_dir),
                        reason: "module_disabled",
                        git: git::inspect_repo(&repo_dir),
                    });
                }
            }
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
                Some(p) if !p.repos_enabled => Some("module_disabled"),
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

fn detect_tasks(
    root: &Path,
    providers: &HashMap<String, ProviderInfo>,
    local_tasks_disabled: bool,
) -> Vec<TaskFinding> {
    let tasks_root = root.join("tasks");
    let mut findings = Vec::new();

    for provider_dir in subdirs(&tasks_root) {
        let display = provider_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if display == "local" {
            // Like remote `module_disabled`: one finding covering the whole
            // local task tree, with open/closed counts, only when turned off.
            if local_tasks_disabled {
                let (files, open, closed) = count_tasks(&provider_dir);
                if files > 0 {
                    findings.push(TaskFinding {
                        provider: "local".to_string(),
                        path: rel(root, &provider_dir),
                        reason: "module_disabled",
                        files,
                        open,
                        closed,
                    });
                }
            }
            continue;
        }

        let info = providers.get(&display);

        // Provider-level reasons cover the whole tree in one finding.
        let provider_reason: Option<&'static str> = match info {
            None => Some("provider_removed"),
            Some(p) if !p.enabled => Some("provider_disabled"),
            Some(p) if !p.tasks_enabled => Some("module_disabled"),
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

/// Count the immediate entries under a module directory (provider subdirs for
/// repos/tasks, label dirs and files for notes/bookmarks). Zero if absent.
fn module_entry_count(root: &Path, module: &str) -> usize {
    std::fs::read_dir(root.join(module))
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .count()
}

/// Detect whole module directories no enabled provider backs anymore. A module
/// is flagged when its `module/` dir exists and is non-empty but
/// [`module_has_backing`] is false.
fn detect_orphaned_modules(root: &Path, cfg: &config::ProjectConfig, scan: impl Fn(&str) -> bool) -> Vec<ModuleFinding> {
    const ALL: &[&str] = &["repos", "tasks", "notes", "bookmarks"];
    let mut findings = Vec::new();
    for &module in ALL {
        if !scan(module) {
            continue;
        }
        let dir = root.join(module);
        if !dir.exists() || module_has_backing(cfg, module) {
            continue;
        }
        let entries = module_entry_count(root, module);
        if entries > 0 {
            findings.push(ModuleFinding {
                module: module.to_string(),
                path: rel(root, &dir),
                reason: "no_provider_backs_module",
                entries,
            });
        }
    }
    findings
}

/// Build the full stale-data report. When `module` is set, only that module is
/// scanned (and local modules are included only via this path).
pub fn report(root: &Path, module: Option<&str>) -> anyhow::Result<TidyReport> {
    let cfg = config::load(root).context("failed to load config")?;
    let providers = build_provider_map(&cfg);

    let scan = |m: &str| module.map_or(true, |only| only == m);

    let repos = if scan("repos") {
        detect_repos(root, &providers, local_module_disabled(&cfg, "repos"))
    } else {
        Vec::new()
    };
    let tasks = if scan("tasks") {
        detect_tasks(root, &providers, local_module_disabled(&cfg, "tasks"))
    } else {
        Vec::new()
    };

    // Local modules are reviewed only on explicit request.
    let local = match module {
        Some(m @ ("notes" | "bookmarks")) => detect_local(root, m)?,
        _ => Vec::new(),
    };

    let modules = detect_orphaned_modules(root, &cfg, scan);

    Ok(TidyReport { repos, tasks, local, modules })
}
