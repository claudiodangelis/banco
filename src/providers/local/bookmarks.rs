use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::context::Label;
use crate::module::{BrowseItem, Module};

pub struct Bookmarks;

fn parse_bookmark_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    let rest = line.strip_prefix("- [")?;
    let (name, rest) = rest.split_once("](")?;
    let url = rest.strip_suffix(')')?;
    if name.is_empty() || url.is_empty() {
        return None;
    }
    Some((name.to_string(), url.to_string()))
}

fn read_group(path: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| parse_bookmark_line(l))
        .collect()
}

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
Bookmarks are organized in group files as markdown lists. Each file (e.g. default.md) \
is a group; entries follow the format `- [name](url)`.\
"
        .to_string()
    }

    fn labels(&self) -> Vec<Label> {
        vec![
            Label {
                name: "group".to_string(),
                kind: "string".to_string(),
                description: "Group name — the .md file the bookmark is added to (default: \"default\")".to_string(),
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

    fn template_paths(&self, _root: &Path) -> Vec<String> {
        vec![]
    }

    fn init(&self, root: &Path) -> anyhow::Result<()> {
        let dir = root.join("bookmarks/local");
        std::fs::create_dir_all(&dir)?;
        let default = dir.join("default.md");
        if !default.exists() {
            std::fs::write(&default, "")?;
        }
        Ok(())
    }

    fn create(&self, root: &Path, name: &str, params: &HashMap<String, String>) -> anyhow::Result<PathBuf> {
        if name.contains('[') || name.contains(']') {
            anyhow::bail!("bookmark name must not contain '[' or ']'");
        }
        let group = params.get("group").map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or("default");
        if sanitize_filename::sanitize(group) != group {
            anyhow::bail!("group name contains characters not valid in a filename: '{}'", group);
        }
        let url = params.get("url").map(|s| s.as_str()).unwrap_or("");
        let dir = root.join("bookmarks/local");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.md", group));
        let mut content = std::fs::read_to_string(&path).unwrap_or_default();
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("- [{}]({})\n", name, url));
        std::fs::write(&path, content)?;
        Ok(path)
    }

    fn browse_items(&self, root: &Path) -> anyhow::Result<Vec<BrowseItem>> {
        let base = root.join("bookmarks/local");
        if !base.exists() {
            return Ok(vec![]);
        }
        let mut entries: Vec<_> = std::fs::read_dir(&base)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.path());
        let mut items = Vec::new();
        for entry in entries {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |e| e == "md") {
                let group = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                for (name, url) in read_group(&path) {
                    let display = if group == "default" { name } else { format!("{}/{}", group, name) };
                    items.push(BrowseItem::default_page(display, url));
                }
            }
        }
        Ok(items)
    }

    fn context(&self, root: &Path) -> anyhow::Result<Vec<Value>> {
        let base = root.join("bookmarks/local");
        if !base.exists() {
            return Ok(vec![]);
        }
        let mut entries: Vec<_> = std::fs::read_dir(&base)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.path());
        let mut items = Vec::new();
        for entry in entries {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |e| e == "md") {
                let group = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                for (name, url) in read_group(&path) {
                    items.push(json!({ "name": name, "group": group, "url": url }));
                }
            }
        }
        Ok(items)
    }

    fn extraneous_paths(&self, root: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let base = root.join("bookmarks/local");
        if !base.exists() {
            return Ok(vec![]);
        }
        Ok(std::fs::read_dir(&base)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir() || p.extension().map_or(true, |e| e != "md"))
            .collect())
    }

    fn root_dirs(&self) -> Vec<&str> {
        vec!["bookmarks"]
    }
}
