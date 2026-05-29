use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::context::Label;
use crate::module::Module;
use super::util::{find_template, label_template_paths};

pub struct Notes;

impl Module for Notes {
    fn name(&self) -> &str {
        "notes"
    }

    fn cli_name(&self) -> &str {
        "note"
    }

    fn describe(&self) -> String {
        "\
## notes/local/
Notes are stored here as markdown files. \
Subdirectories act as labels/tags and can be nested.\
"
        .to_string()
    }

    fn labels(&self) -> Vec<Label> {
        vec![Label {
            name: "label".to_string(),
            kind: "string".to_string(),
            description: "Optional nested path used as a tag (e.g. meetings/2026)".to_string(),
            values: None,
        }]
    }

    fn template_paths(&self, root: &Path) -> Vec<String> {
        label_template_paths("notes/local", root)
    }

    fn init(&self, root: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(root.join("notes/local"))?;
        Ok(())
    }

    fn create(&self, root: &Path, name: &str, params: &HashMap<String, String>) -> anyhow::Result<PathBuf> {
        let label = params.get("label").map(|s| s.as_str()).unwrap_or("");
        let dir = if label.is_empty() {
            root.join("notes/local")
        } else {
            root.join("notes/local").join(label)
        };
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.md", name));
        let content = find_template(root, "notes/local", label).unwrap_or_default();
        std::fs::write(&path, content)?;
        Ok(path)
    }

    fn context(&self, root: &Path) -> anyhow::Result<Vec<Value>> {
        let base = root.join("notes/local");
        if !base.exists() {
            return Ok(vec![]);
        }

        let mut items = Vec::new();
        for entry in WalkDir::new(&base).min_depth(1).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |e| e == "md") {
                let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                let label = path
                    .parent()
                    .and_then(|p| p.strip_prefix(&base).ok())
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string();
                items.push(json!({ "name": title, "label": label }));
            }
        }
        Ok(items)
    }

    fn root_dirs(&self) -> Vec<&str> {
        vec!["notes"]
    }
}
