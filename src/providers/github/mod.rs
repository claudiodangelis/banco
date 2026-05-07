mod client;
mod repos;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use walkdir::WalkDir;

use regex::Regex;

use crate::config::ProviderEntry;
use crate::context::ModuleContext;
use crate::module::Module;
use crate::provider::{ConfigParam, ConfigParamKind, Provider};
use crate::template::find_template;

use client::{GitHubClient, Issue};
use repos::GitHubRepos;

pub struct GitHubProvider {
    entry: ProviderEntry,
}

impl GitHubProvider {
    pub fn new(entry: ProviderEntry) -> Self {
        Self { entry }
    }

    pub fn available_config_schema() -> Vec<ConfigParam> {
        vec![
            ConfigParam {
                name: "api_key",
                description: "GitHub personal access token",
                kind: ConfigParamKind::String,
                required: true,
            },
            ConfigParam {
                name: "host",
                description: "GitHub instance URL (default: https://github.com) — set for GitHub Enterprise Server",
                kind: ConfigParamKind::String,
                required: false,
            },
            ConfigParam {
                name: "projects",
                description: "Explicit project paths to sync (owner/repo) — mutually exclusive with projects_pattern",
                kind: ConfigParamKind::List,
                required: false,
            },
            ConfigParam {
                name: "projects_pattern",
                description: "Regex to match project paths (owner/repo) — mutually exclusive with projects",
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

    fn client(&self) -> GitHubClient {
        let token = self.entry.get_str("api_key");
        let base_url = match self.entry.get_str("host") {
            Some(host) => format!("{}/api/v3", host.trim_end_matches('/')),
            None => "https://api.github.com".to_string(),
        };
        GitHubClient::new(&base_url, token)
    }

    fn resolved_projects(&self, client: &GitHubClient) -> anyhow::Result<Vec<String>> {
        let explicit = self.entry.get_list("projects");
        let pattern = self.entry.get_str("projects_pattern");

        match (explicit.is_empty(), pattern) {
            (false, Some(_)) => {
                anyhow::bail!("`projects` and `projects_pattern` are mutually exclusive")
            }
            (false, None) => Ok(explicit),
            (true, Some(pat)) => {
                println!("  info: projects_pattern requires fetching all accessible repos, which may take a while");
                let re = Regex::new(&format!("^(?:{})$", pat))
                    .map_err(|e| anyhow::anyhow!("invalid projects_pattern: {}", e))?;
                let all = client.all_repos()?;
                Ok(all
                    .into_iter()
                    .filter(|r| re.is_match(&r.full_name))
                    .map(|r| r.full_name)
                    .collect())
            }
            (true, None) => anyhow::bail!("either `projects` or `projects_pattern` must be set"),
        }
    }

    fn github_repos(&self, root: &Path) -> GitHubRepos {
        GitHubRepos {
            repos_root: self.repos_root(root),
        }
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
    if let Some(body) = &issue.body {
        if !body.is_empty() {
            out.push_str(body);
            out.push('\n');
        }
    }
    out
}

fn apply_issue(
    issue: &Issue,
    col_dir: &Path,
    existing: &mut HashMap<u64, PathBuf>,
    template: Option<&str>,
) -> anyhow::Result<()> {
    let filename = format!("{:04} - {}.md", issue.number, sanitize(&issue.title));
    let expected = col_dir.join(&filename);

    match existing.get(&issue.number) {
        None => {
            if !expected.exists() {
                let content = template
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| format_issue(issue));
                std::fs::write(&expected, content)?;
                existing.insert(issue.number, expected);
            }
        }
        Some(current) => {
            if current != &expected && !expected.exists() {
                std::fs::rename(current, &expected)?;
                existing.insert(issue.number, expected);
            }
        }
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

impl Provider for GitHubProvider {
    fn name(&self) -> &str {
        self.entry.display_name()
    }

    fn init(&self, root: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.tasks_root(root))?;
        self.github_repos(root).init(root)?;
        Ok(())
    }

    fn sync(&self, root: &Path) -> anyhow::Result<()> {
        let client = self.client();
        let projects = self.resolved_projects(&client)?;

        let template_base = format!("tasks/{}", self.entry.display_name());

        for owner_repo in &projects {
            let owner = owner_repo.split('/').next().unwrap_or(owner_repo);
            let repo_name = owner_repo.rsplit('/').next().unwrap_or(owner_repo);
            let task_dir = self.tasks_root(root).join(owner).join(repo_name);

            let open_col = task_dir.join("1-open");
            let closed_col = task_dir.join("2-closed");
            std::fs::create_dir_all(&open_col)?;
            std::fs::create_dir_all(&closed_col)?;

            let open_tpl = find_template(
                root,
                &template_base,
                &format!("{}/{}/1-open", owner, repo_name),
            );
            let closed_tpl = find_template(
                root,
                &template_base,
                &format!("{}/{}/2-closed", owner, repo_name),
            );

            let mut existing_open = scan_col(&open_col);
            for issue in client.issues_open(owner_repo)? {
                if issue.pull_request.is_some() {
                    continue;
                }
                apply_issue(&issue, &open_col, &mut existing_open, open_tpl.as_deref())?;
            }

            let mut existing_closed = scan_col(&closed_col);
            for issue in client.issues_closed(owner_repo)? {
                if issue.pull_request.is_some() {
                    continue;
                }
                apply_issue(
                    &issue,
                    &closed_col,
                    &mut existing_closed,
                    closed_tpl.as_deref(),
                )?;
            }

            println!("  synced issues for {}", owner_repo);
        }

        if !projects.is_empty() {
            self.github_repos(root).sync(&client, &projects)?;
        }

        Ok(())
    }

    fn context(&self, root: &Path) -> anyhow::Result<Vec<ModuleContext>> {
        let issues_base = self.tasks_root(root);
        let mut issue_items: Vec<Value> = Vec::new();

        if issues_base.exists() {
            for entry in WalkDir::new(&issues_base)
                .min_depth(4)
                .max_depth(4)
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
                    if parts.len() == 4 {
                        let title = Path::new(parts[3])
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(parts[3]);
                        issue_items.push(json!({
                            "owner":  parts[0],
                            "repo":   parts[1],
                            "column": parts[2],
                            "name":   title,
                        }));
                    }
                }
            }
        }

        let repo_items = self.github_repos(root).context(root)?;

        Ok(vec![
            ModuleContext {
                name: "tasks".to_string(),
                labels: vec![],
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
