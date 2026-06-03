mod client;
mod repos;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use walkdir::WalkDir;
use regex::Regex;

use crate::config::ProviderEntry;
use crate::context::{Label, ModuleContext};
use crate::module::{BrowseItem, Module};
use crate::provider::{ConfigParam, ConfigParamKind, Provider, SyncOpts};
use crate::sync_state;
use crate::template::find_template;

use crate::providers::frontmatter;
use crate::providers::git::{normalize_git_url, read_git_remote_url};

use client::{GitLabClient, Issue};
use repos::GitLabRepos;

pub struct GitLabProvider {
    entry: ProviderEntry,
}

impl GitLabProvider {
    pub fn new(entry: ProviderEntry) -> Self {
        Self { entry }
    }

    pub fn available_config_schema() -> Vec<ConfigParam> {
        vec![
            ConfigParam {
                name: "api_key",
                description: "GitLab personal access token",
                kind: ConfigParamKind::String,
                required: true,
            },
            ConfigParam {
                name: "host",
                description: "GitLab instance URL (default: https://gitlab.com)",
                kind: ConfigParamKind::String,
                required: false,
            },
            ConfigParam {
                name: "sync_issues",
                description: "Sync issues as tasks (default: true)",
                kind: ConfigParamKind::Bool,
                required: false,
            },
            ConfigParam {
                name: "projects",
                description: "Explicit project paths to sync (namespace/project) — mutually exclusive with projects_pattern",
                kind: ConfigParamKind::List,
                required: false,
            },
            ConfigParam {
                name: "projects_pattern",
                description: "Regex to match project paths (namespace/project) — mutually exclusive with projects",
                kind: ConfigParamKind::String,
                required: false,
            },
        ]
    }

    fn tasks_root(&self, root: &Path) -> PathBuf {
        root.join("tasks").join(self.entry.display_name())
    }

    fn repos_root(&self, root: &Path) -> PathBuf {
        root.join("repos").join(self.entry.display_name())
    }

    fn client(&self) -> GitLabClient {
        let host = self
            .entry
            .get_str("host")
            .unwrap_or_else(|| "https://gitlab.com".to_string());
        let token = self.entry.get_str("api_key");
        GitLabClient::new(&host, token)
    }

    fn resolved_projects(&self, client: &GitLabClient) -> anyhow::Result<Vec<String>> {
        let explicit = self.entry.get_list("projects");
        let pattern = self.entry.get_str("projects_pattern");

        match (explicit.is_empty(), pattern) {
            (false, Some(_)) => anyhow::bail!("`projects` and `projects_pattern` are mutually exclusive"),
            (false, None) => Ok(explicit),
            (true, Some(pat)) => {
                println!("  info: projects_pattern requires fetching all accessible projects, which may take a while");
                let re = Regex::new(&format!("^(?:{})$", pat))
                    .map_err(|e| anyhow::anyhow!("invalid projects_pattern: {}", e))?;
                let all = client.all_projects()?;
                Ok(all
                    .into_iter()
                    .filter(|p| re.is_match(&p.path_with_namespace))
                    .map(|p| p.path_with_namespace)
                    .collect())
            }
            (true, None) => anyhow::bail!("either `projects` or `projects_pattern` must be set"),
        }
    }

    fn gitlab_repos(&self, root: &Path) -> GitLabRepos {
        GitLabRepos { repos_root: self.repos_root(root) }
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .take(80)
        .collect()
}

fn format_issue(issue: &Issue) -> String {
    let mut out = format!("# {}\n\n", issue.title);
    if let Some(desc) = &issue.description {
        if !desc.is_empty() {
            out.push_str(desc);
            out.push('\n');
        }
    }
    out
}

fn apply_issue(
    issue: &Issue,
    status: &str,
    task_dir: &Path,
    existing: &mut HashMap<u64, PathBuf>,
    template: Option<&str>,
) -> anyhow::Result<()> {
    let filename = format!("{:04} - {}.md", issue.iid, sanitize(&issue.title));
    let expected = task_dir.join(&filename);

    match existing.get(&issue.iid) {
        None => {
            if !expected.exists() {
                let content = template
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| format_issue(issue));
                std::fs::write(&expected, &content)?;
                existing.insert(issue.iid, expected.clone());
            }
        }
        Some(current) => {
            if current != &expected && !expected.exists() {
                std::fs::rename(current, &expected)?;
                existing.insert(issue.iid, expected.clone());
            }
        }
    }

    let actual = existing.get(&issue.iid).cloned().unwrap_or(expected);
    if actual.exists() {
        frontmatter::apply(&actual, status, &issue.labels)?;
    }
    Ok(())
}

fn scan_col(col_dir: &Path) -> HashMap<u64, PathBuf> {
    let mut map = HashMap::new();
    if !col_dir.exists() {
        return map;
    }
    for entry in std::fs::read_dir(col_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            if let Some(n) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.splitn(2, " - ").next())
                .and_then(|n| n.trim().parse::<u64>().ok())
            {
                map.insert(n, path);
            }
        }
    }
    map
}

impl Provider for GitLabProvider {
    fn name(&self) -> &str {
        self.entry.display_name()
    }

    fn init(&self, root: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.tasks_root(root))?;
        self.gitlab_repos(root).init(root)?;
        Ok(())
    }

    fn sync(&self, root: &Path, opts: &SyncOpts) -> anyhow::Result<()> {
        let client = self.client();
        let mut projects = self.resolved_projects(&client)?;

        if let Some(ref pat) = opts.pattern {
            let re = Regex::new(&format!("^(?:{})$", pat))
                .map_err(|e| anyhow::anyhow!("invalid --pattern: {}", e))?;
            projects.retain(|p| re.is_match(p));
        }

        let sync_tasks = opts.module.as_deref().map_or(true, |m| m == "tasks");
        let sync_repos = opts.module.as_deref().map_or(true, |m| m == "repos");

        let sync_issues = self.entry.get_bool("sync_issues", true);
        let template_base = "tasks";
        let synced_at = sync_state::now();

        if sync_tasks && sync_issues {
            for namespace_project in &projects {
                let project = client.project(namespace_project)?;
                let task_dir = self.tasks_root(root).join(&project.path);
                std::fs::create_dir_all(&task_dir)?;

                let tpl = find_template(root, template_base, &format!("{}/{}", self.entry.display_name(), project.path));

                let mut existing = scan_col(&task_dir);
                let mut fetched_numbers: std::collections::HashSet<u64> = std::collections::HashSet::new();
                for issue in client.issues_open(project.id, None)? {
                    fetched_numbers.insert(issue.iid);
                    apply_issue(&issue, "open", &task_dir, &mut existing, tpl.as_deref())?;
                }

                // Reconcile: local non-closed tasks absent from the open set were closed on remote.
                for (iid, path) in &existing {
                    if fetched_numbers.contains(iid) {
                        continue;
                    }
                    let content = std::fs::read_to_string(path)?;
                    let (fm, _) = frontmatter::parse(&content);
                    if let Some(fm) = fm {
                        if fm.status != "closed" {
                            frontmatter::apply(path, "closed", &fm.tags)?;
                        }
                    }
                }

                println!("  synced issues for {}", namespace_project);
            }
        }

        if sync_repos && !projects.is_empty() {
            self.gitlab_repos(root).sync(&client, &projects)?;
        }

        sync_state::write(root, self.entry.display_name(), &synced_at)?;
        Ok(())
    }

    fn browse_modules(&self, root: &Path) -> anyhow::Result<Vec<(String, Vec<BrowseItem>)>> {
        let mut result = Vec::new();

        // Repos: read git remote origin from each cloned repo; also build a
        // project_name -> base_url map so task URLs can be derived from it.
        let repos_base = self.repos_root(root);
        let mut repo_url_map: HashMap<String, String> = HashMap::new();
        let mut repo_items: Vec<BrowseItem> = Vec::new();
        if repos_base.exists() {
            let mut entries: Vec<_> = std::fs::read_dir(&repos_base)?
                .filter_map(|e| e.ok())
                .collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                    if let Some(remote_url) = read_git_remote_url(&path) {
                        let base = normalize_git_url(&remote_url);
                        repo_url_map.insert(name.clone(), base.clone());
                        repo_items.push(BrowseItem {
                            display: name,
                            pages: vec![
                                ("Repository".to_string(),     base.clone()),
                                ("Merge Requests".to_string(), format!("{}/-/merge_requests", base)),
                                ("Pipelines".to_string(),      format!("{}/-/pipelines", base)),
                            ],
                        });
                    }
                }
            }
        }
        if !repo_items.is_empty() {
            result.push(("repos".to_string(), repo_items));
        }

        // Tasks: derive issue URL from tasks/{provider}/{project}/{n} - {title}.md.
        // Namespace is resolved via the cloned repo's remote URL; skip items where
        // no matching repo is found (namespace would be unknown).
        let tasks_base = self.tasks_root(root);
        let mut task_items: Vec<BrowseItem> = Vec::new();
        if tasks_base.exists() {
            for entry in WalkDir::new(&tasks_base)
                .min_depth(2).max_depth(2)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |e| e == "md") {
                    let rel = path.strip_prefix(&tasks_base).ok()
                        .and_then(|p| p.to_str()).unwrap_or("");
                    let parts: Vec<&str> = rel.split('/').collect();
                    if parts.len() == 2 {
                        let project = parts[0];
                        let stem = Path::new(parts[1]).file_stem()
                            .and_then(|s| s.to_str()).unwrap_or(parts[1]);
                        let mut split = stem.splitn(2, " - ");
                        if let (Some(num_str), Some(title)) = (split.next(), split.next()) {
                            if let Ok(num) = num_str.trim().parse::<u64>() {
                                if let Some(base_url) = repo_url_map.get(project) {
                                    let display = format!("{} #{} {}", project, num, title);
                                    let url = format!("{}/-/issues/{}", base_url, num);
                                    task_items.push(BrowseItem::default_page(display, url));
                                }
                            }
                        }
                    }
                }
            }
        }
        if !task_items.is_empty() {
            result.push(("tasks".to_string(), task_items));
        }

        Ok(result)
    }

    fn context(&self, root: &Path) -> anyhow::Result<Vec<ModuleContext>> {
        let issues_base = self.tasks_root(root);
        let mut issue_items: Vec<Value> = Vec::new();

        if issues_base.exists() {
            for entry in WalkDir::new(&issues_base)
                .min_depth(2)
                .max_depth(2)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |e| e == "md") {
                    let rel = path
                        .strip_prefix(&issues_base)
                        .ok()
                        .and_then(|p| p.to_str())
                        .unwrap_or("");
                    let parts: Vec<&str> = rel.split('/').collect();
                    if parts.len() == 2 {
                        let title = Path::new(parts[1])
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(parts[1]);
                        let fm = std::fs::read_to_string(path)
                            .ok()
                            .and_then(|c| frontmatter::parse(&c).0);
                        let status = fm.as_ref().map(|f| f.status.as_str()).unwrap_or("");
                        let tags: Vec<&str> = fm
                            .as_ref()
                            .map(|f| f.tags.iter().map(|t| t.as_str()).collect())
                            .unwrap_or_default();
                        issue_items.push(json!({
                            "project": parts[0],
                            "name":    title,
                            "status":  status,
                            "tags":    tags,
                        }));
                    }
                }
            }
        }

        let repo_items = self.gitlab_repos(root).context(root)?;

        Ok(vec![
            ModuleContext {
                name: "tasks".to_string(),
                labels: vec![
                    Label {
                        name: "status".to_string(),
                        kind: "string".to_string(),
                        description: "Issue state".to_string(),
                        values: Some(vec!["open".to_string(), "closed".to_string()]),
                    },
                    Label {
                        name: "tags".to_string(),
                        kind: "list".to_string(),
                        description: "Issue labels".to_string(),
                        values: None,
                    },
                ],
                items: issue_items,
            },
            ModuleContext {
                name: "repos".to_string(),
                labels: vec![],
                items: repo_items,
            },
        ])
    }
}
