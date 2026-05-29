use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::context::Label;
use crate::module::Module;
use super::util::find_template;

pub struct Tasks;

const STATUSES: &[&str] = &["backlog", "doing", "done"];

fn next_task_number(root: &Path) -> u32 {
    let base = root.join("tasks/local");
    let max = STATUSES.iter()
        .filter_map(|s| std::fs::read_dir(base.join(s)).ok())
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            e.file_name()
                .to_string_lossy()
                .splitn(2, ' ')
                .next()
                .and_then(|n| n.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);
    max + 1
}

impl Module for Tasks {
    fn name(&self) -> &str {
        "tasks"
    }

    fn cli_name(&self) -> &str {
        "task"
    }

    fn describe(&self) -> String {
        "\
## tasks/local/
Tasks are stored here as markdown files, organized by status:

- `backlog/` — tasks waiting to be started
- `doing/`    — tasks currently in progress
- `done/`     — completed tasks\
"
        .to_string()
    }

    fn labels(&self) -> Vec<Label> {
        vec![Label {
            name: "status".to_string(),
            kind: "enum".to_string(),
            description: "Status of the task".to_string(),
            values: Some(STATUSES.iter().map(|s| s.to_string()).collect()),
        }]
    }

    fn template_paths(&self, _root: &Path) -> Vec<String> {
        vec!["tasks/local".to_string()]
    }

    fn init(&self, root: &Path) -> anyhow::Result<()> {
        for status in STATUSES {
            std::fs::create_dir_all(root.join("tasks/local").join(status))?;
        }
        Ok(())
    }

    fn create(&self, root: &Path, name: &str, params: &HashMap<String, String>) -> anyhow::Result<PathBuf> {
        let status = params.get("status").map(|s| s.as_str()).unwrap_or("backlog");
        if !STATUSES.contains(&status) {
            anyhow::bail!("invalid status '{}'; must be one of: {}", status, STATUSES.join(", "));
        }
        let next = next_task_number(root);
        let dir = root.join("tasks/local").join(status);
        let path = dir.join(format!("{:04} - {}.md", next, name));
        let content = find_template(root, "tasks/local", "").unwrap_or_default();
        std::fs::write(&path, content)?;
        Ok(path)
    }

    fn context(&self, root: &Path) -> anyhow::Result<Vec<Value>> {
        let base = root.join("tasks/local");
        if !base.exists() {
            return Ok(vec![]);
        }

        let mut items = Vec::new();
        for status in STATUSES {
            let dir = base.join(status);
            if !dir.exists() {
                continue;
            }
            for entry in std::fs::read_dir(&dir)? {
                let path = entry?.path();
                if path.is_file() && path.extension().map_or(false, |e| e == "md") {
                    let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                    items.push(json!({ "name": title, "status": status }));
                }
            }
        }
        Ok(items)
    }

    fn root_dirs(&self) -> Vec<&str> {
        vec!["tasks"]
    }

    fn extraneous_paths(&self, root: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let base = root.join("tasks/local");
        if !base.exists() {
            return Ok(vec![]);
        }
        let mut extra = Vec::new();
        for entry in std::fs::read_dir(&base)? {
            let path = entry?.path();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if path.is_dir() {
                if !STATUSES.contains(&name) {
                    extra.push(path);
                } else {
                    for sub in std::fs::read_dir(&path)? {
                        let sub_path = sub?.path();
                        if sub_path.is_file() && sub_path.extension().map_or(true, |e| e != "md") {
                            extra.push(sub_path);
                        }
                    }
                }
            } else if path.is_file() {
                extra.push(path);
            }
        }
        Ok(extra)
    }
}
