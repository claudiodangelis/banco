mod claude;
mod frontmatter;
mod scan;
pub mod schema;

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::config::ProviderEntry;
use crate::context::{Label, ModuleContext};
use crate::module::BrowseItem;
use crate::provider::{ConfigParam, ConfigParamKind, Provider, SyncOpts};
use crate::sync_state;
use crate::template::find_template;

pub struct JiraProvider {
    entry: ProviderEntry,
}

impl JiraProvider {
    pub fn new(entry: ProviderEntry) -> Self {
        Self { entry }
    }

    pub fn available_config_schema() -> Vec<ConfigParam> {
        vec![
            ConfigParam {
                name: "host",
                description: "JIRA instance URL (e.g. https://yourorg.atlassian.net) — used to construct browse URLs",
                kind: ConfigParamKind::String,
                required: true,
            },
            ConfigParam {
                name: "project",
                description: "Project key to sync (e.g. ENG)",
                kind: ConfigParamKind::String,
                required: true,
            },
            ConfigParam {
                name: "labels",
                description: "Labels to filter issues by (leave empty to fetch all)",
                kind: ConfigParamKind::List,
                required: false,
            },
            ConfigParam {
                name: "agent_backend",
                description: "Agent backend to use for fetching data (claude)",
                kind: ConfigParamKind::String,
                required: true,
            },
        ]
    }

    fn tasks_root(&self, root: &Path) -> PathBuf {
        root.join("tasks").join(self.entry.display_name())
    }

    fn host(&self) -> String {
        self.entry
            .get_str("host")
            .unwrap_or_default()
            .trim_end_matches('/')
            .to_string()
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

impl Provider for JiraProvider {
    fn name(&self) -> &str {
        self.entry.display_name()
    }

    fn init(&self, root: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.tasks_root(root))?;
        Ok(())
    }

    fn sync(&self, root: &Path, opts: &SyncOpts) -> anyhow::Result<()> {
        if opts.module.as_deref() == Some("repos") {
            println!("  skipping: jira provider has no repos module");
            return Ok(());
        }

        let project = self.entry.get_str("project")
            .ok_or_else(|| anyhow::anyhow!("`project` is required in the jira provider config"))?;
        let labels = self.entry.get_list("labels");
        let backend = self.entry.get_str("agent_backend")
            .unwrap_or_else(|| "claude".to_string());

        anyhow::ensure!(
            backend == "claude",
            "unsupported agent_backend '{}'; only 'claude' is supported",
            backend
        );

        let since = sync_state::read(root, self.entry.display_name());
        let synced_at = sync_state::now();

        println!("  fetching issues from project {} via {}...", project, backend);

        let issues = claude::fetch_issues(&project, &labels, since.as_deref())?;
        schema::validate(&issues)?;

        let task_dir = self.tasks_root(root);
        std::fs::create_dir_all(&task_dir)?;

        let tpl = find_template(root, "tasks", self.entry.display_name());

        let mut existing = scan::scan(&task_dir);

        for issue in &issues {
            let filename = format!("{} - {}.md", issue.id, sanitize(&issue.title));
            let expected = task_dir.join(&filename);

            match existing.get(&issue.id) {
                None => {
                    if !expected.exists() {
                        let content = tpl.clone().unwrap_or_default();
                        std::fs::write(&expected, &content)?;
                        existing.insert(issue.id.clone(), expected.clone());
                    }
                }
                Some(current) => {
                    if current != &expected && !expected.exists() {
                        std::fs::rename(current, &expected)?;
                        existing.insert(issue.id.clone(), expected.clone());
                    }
                }
            }

            let actual = existing.get(&issue.id).cloned().unwrap_or(expected);
            if actual.exists() {
                frontmatter::apply(
                    &actual,
                    &issue.status,
                    &issue.issue_type,
                    issue.parent_id.as_deref(),
                )?;
            }
        }

        // Reconcile: local non-done tasks absent from the fetched set were completed on remote.
        let fetched_ids: std::collections::HashSet<&str> =
            issues.iter().map(|i| i.id.as_str()).collect();
        for (id, path) in &existing {
            if fetched_ids.contains(id.as_str()) {
                continue;
            }
            let content = std::fs::read_to_string(path)?;
            let (fm, _) = frontmatter::parse(&content);
            if let Some(fm) = fm {
                if fm.status != "Done" {
                    frontmatter::apply(path, "Done", &fm.issue_type, fm.parent_id.as_deref())?;
                }
            }
        }

        sync_state::write(root, self.entry.display_name(), &synced_at)?;
        println!("  synced {} issues for project {}", issues.len(), project);
        Ok(())
    }

    fn context(&self, root: &Path) -> anyhow::Result<Vec<ModuleContext>> {
        let tasks_base = self.tasks_root(root);
        let mut items: Vec<Value> = Vec::new();

        if tasks_base.exists() {
            for entry in WalkDir::new(&tasks_base)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |e| e == "md") {
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    let mut split = stem.splitn(2, " - ");
                    let id = split.next().unwrap_or("").trim().to_string();
                    let title = split.next().unwrap_or(stem).to_string();

                    let content = std::fs::read_to_string(path).unwrap_or_default();
                    let (fm, _) = frontmatter::parse(&content);

                    let status = fm.as_ref().map(|f| f.status.as_str()).unwrap_or("").to_string();
                    let issue_type = fm.as_ref().map(|f| f.issue_type.as_str()).unwrap_or("").to_string();
                    let parent_id: Option<String> = fm.and_then(|f| f.parent_id);

                    items.push(json!({
                        "id":        id,
                        "name":      title,
                        "status":    status,
                        "type":      issue_type,
                        "parent_id": parent_id,
                    }));
                }
            }
        }

        Ok(vec![ModuleContext {
            name: "tasks".to_string(),
            labels: vec![
                Label {
                    name: "status".to_string(),
                    kind: "string".to_string(),
                    description: "Issue status".to_string(),
                    values: None,
                },
                Label {
                    name: "type".to_string(),
                    kind: "string".to_string(),
                    description: "Issue type (Epic, Story, Task, Sub-task, Bug, ...)".to_string(),
                    values: None,
                },
                Label {
                    name: "parent_id".to_string(),
                    kind: "string".to_string(),
                    description: "Parent issue ID, if any".to_string(),
                    values: None,
                },
            ],
            items,
        }])
    }

    fn browse_modules(&self, root: &Path) -> anyhow::Result<Vec<(String, Vec<BrowseItem>)>> {
        let tasks_base = self.tasks_root(root);
        let host = self.host();
        if host.is_empty() {
            return Ok(vec![]);
        }

        let mut items: Vec<BrowseItem> = Vec::new();

        if tasks_base.exists() {
            for entry in WalkDir::new(&tasks_base)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |e| e == "md") {
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    let mut split = stem.splitn(2, " - ");
                    if let (Some(id), Some(title)) = (split.next(), split.next()) {
                        let id = id.trim();
                        let url = format!("{}/browse/{}", host, id);
                        let display = format!("{} {}", id, title);
                        items.push(BrowseItem::default_page(display, url));
                    }
                }
            }
        }

        if items.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![("tasks".to_string(), items)])
    }
}
