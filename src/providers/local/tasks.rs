use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};

use crate::context::Param;
use crate::module::Module;
use super::util::find_template;

pub struct Tasks;

const STATUSES: &[&str] = &["awaiting", "doing", "done"];

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

- `awaiting/` — tasks waiting to be started
- `doing/`    — tasks currently in progress
- `done/`     — completed tasks\
"
        .to_string()
    }

    fn parameters(&self) -> Vec<Param> {
        vec![Param {
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

    fn create(&self, root: &Path, name: &str, params: &HashMap<String, String>) -> anyhow::Result<()> {
        let status = params.get("status").map(|s| s.as_str()).unwrap_or("awaiting");
        if !STATUSES.contains(&status) {
            anyhow::bail!("invalid status '{}'; must be one of: {}", status, STATUSES.join(", "));
        }
        let next = next_task_number(root);
        let dir = root.join("tasks/local").join(status);
        let content = find_template(root, "tasks/local", "").unwrap_or_default();
        std::fs::write(dir.join(format!("{:04} - {}.md", next, name)), content)?;
        Ok(())
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
}
