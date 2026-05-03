use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::context::Label;
use crate::module::Module;
use super::util::{find_template, label_template_paths};

pub struct Bookmarks;

impl Module for Bookmarks {
    fn name(&self) -> &str {
        "bookmarks"
    }

    fn cli_name(&self) -> &str {
        "bookmark"
    }

    fn describe(&self) -> String {
        "\
## bookmarks/local/
Bookmarks are stored here as markdown files. \
Subdirectories act as labels/tags and can be nested.\
"
        .to_string()
    }

    fn labels(&self) -> Vec<Label> {
        vec![
            Label {
                name: "label".to_string(),
                kind: "string".to_string(),
                description: "Optional nested path used as a tag (e.g. tools/rust)".to_string(),
                values: None,
            },
            Label {
                name: "url".to_string(),
                kind: "string".to_string(),
                description: "URL of the bookmarked resource".to_string(),
                values: None,
            },
        ]
    }

    fn template_paths(&self, root: &Path) -> Vec<String> {
        label_template_paths("bookmarks/local", root)
    }

    fn init(&self, root: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(root.join("bookmarks/local"))?;
        Ok(())
    }

    fn create(&self, root: &Path, name: &str, params: &HashMap<String, String>) -> anyhow::Result<PathBuf> {
        let label = params.get("label").map(|s| s.as_str()).unwrap_or("");
        let url = params.get("url").map(|s| s.as_str()).unwrap_or("");
        let dir = if label.is_empty() {
            root.join("bookmarks/local")
        } else {
            root.join("bookmarks/local").join(label)
        };
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.md", name));
        let template = find_template(root, "bookmarks/local", label).unwrap_or_default();
        let content = if template.is_empty() {
            url.to_string()
        } else {
            format!("{}\n\n{}", url, template)
        };
        std::fs::write(&path, content)?;
        Ok(path)
    }

    fn context(&self, root: &Path) -> anyhow::Result<Vec<Value>> {
        let base = root.join("bookmarks/local");
        if !base.exists() {
            return Ok(vec![]);
        }

        let mut items = Vec::new();
        for entry in WalkDir::new(&base).min_depth(1).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |e| e == "md") {
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                let label = path
                    .parent()
                    .and_then(|p| p.strip_prefix(&base).ok())
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string();
                let url = std::fs::read_to_string(path)
                    .ok()
                    .and_then(|c| c.lines().next().map(|l| l.to_string()))
                    .unwrap_or_default();
                items.push(json!({ "name": name, "label": label, "url": url }));
            }
        }
        Ok(items)
    }
}
