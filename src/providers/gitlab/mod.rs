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
            .unwrap_or("https://gitlab.com")
            .to_string();
        let token = self.entry.get_str("api_key").map(|s| s.to_string());
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

/// Scans a board directory and returns a map of iid → current file path.
/// Files are expected at board_dir/<column>/<iid> - <title>.md (depth 2).
fn scan_board(board_dir: &Path) -> HashMap<u64, PathBuf> {
    let mut map = HashMap::new();
    if !board_dir.exists() {
        return map;
    }
    for entry in WalkDir::new(board_dir)
        .min_depth(2)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            if let Some(iid) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.splitn(2, " - ").next())
                .and_then(|n| n.trim().parse::<u64>().ok())
            {
                map.insert(iid, path.to_path_buf());
            }
        }
    }
    map
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

fn slug(s: &str) -> String {
    let raw: String = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    raw.split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn format_issue(issue: &Issue) -> String {
    let mut out = String::from(&format!("# {}\n\n", issue.title));
    if let Some(desc) = &issue.description {
        if !desc.is_empty() {
            out.push_str(desc);
            out.push('\n');
        }
    }
    out
}

/// Applies a single issue to the board directory using the existing-file index.
/// - New issue: create file using template if available, otherwise auto-generated content.
/// - Renamed or moved: rename/move the file, never touch its content.
/// - Unchanged: do nothing.
/// Never writes to a path that already exists on disk.
fn apply_issue(
    issue: &Issue,
    col_dir: &Path,
    existing: &mut HashMap<u64, PathBuf>,
    template: Option<&str>,
) -> anyhow::Result<()> {
    let filename = format!("{:04} - {}.md", issue.iid, sanitize(&issue.title));
    let expected = col_dir.join(&filename);

    match existing.get(&issue.iid) {
        None => {
            if !expected.exists() {
                let content = template.map(|t| t.to_string()).unwrap_or_else(|| format_issue(issue));
                std::fs::write(&expected, content)?;
                existing.insert(issue.iid, expected);
            }
        }
        Some(current) => {
            if current != &expected && !expected.exists() {
                std::fs::rename(current, &expected)?;
                existing.insert(issue.iid, expected);
            }
        }
    }
    Ok(())
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

    fn sync(&self, root: &Path) -> anyhow::Result<()> {
        let client = self.client();
        let projects = self.resolved_projects(&client)?;

        for namespace_project in &projects {
            let project = client.project(&namespace_project)?;
            let boards = client.boards(project.id)?;

            for board in &boards {
                let board_dir = self
                    .tasks_root(root)
                    .join(&project.path)
                    .join(slug(&board.name));
                let lists = client.board_lists(project.id, board.id)?;
                let total = lists.len() + 2;
                let width = total.to_string().len();

                let mut existing = scan_board(&board_dir);

                let template_base =
                    format!("tasks/{}", self.entry.display_name());
                let board_slug = slug(&board.name);

                // 1-open
                let excluded = lists
                    .iter()
                    .map(|l| l.label.name.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                let open_col = format!("{:0width$}-open", 1, width = width);
                let open_dir = board_dir.join(&open_col);
                std::fs::create_dir_all(&open_dir)?;
                let open_tpl = find_template(
                    root,
                    &template_base,
                    &format!("{}/{}/{}", project.path, board_slug, open_col),
                );
                for issue in &client.issues_opened(project.id, &excluded)? {
                    apply_issue(issue, &open_dir, &mut existing, open_tpl.as_deref())?;
                }

                // label-based columns
                for (i, list) in lists.iter().enumerate() {
                    let col_name = format!(
                        "{:0width$}-{}",
                        i + 2,
                        slug(&list.label.name),
                        width = width
                    );
                    let col_dir = board_dir.join(&col_name);
                    std::fs::create_dir_all(&col_dir)?;
                    let tpl = find_template(
                        root,
                        &template_base,
                        &format!("{}/{}/{}", project.path, board_slug, col_name),
                    );
                    for issue in &client.issues_by_label(project.id, &list.label.name)? {
                        apply_issue(issue, &col_dir, &mut existing, tpl.as_deref())?;
                    }
                }

                // N-closed
                let closed_col = format!("{:0width$}-closed", total, width = width);
                let closed_dir = board_dir.join(&closed_col);
                std::fs::create_dir_all(&closed_dir)?;
                let closed_tpl = find_template(
                    root,
                    &template_base,
                    &format!("{}/{}/{}", project.path, board_slug, closed_col),
                );
                for issue in &client.issues_closed(project.id)? {
                    apply_issue(issue, &closed_dir, &mut existing, closed_tpl.as_deref())?;
                }

                println!("  synced board '{}' for {}", board.name, namespace_project);
            }
        }

        if !projects.is_empty() {
            self.gitlab_repos(root).sync(&client, &projects)?;
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
                            "project": parts[0],
                            "board": parts[1],
                            "column": parts[2],
                            "name": title,
                        }));
                    }
                }
            }
        }

        let repo_items = self.gitlab_repos(root).context(root)?;

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
