use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};

use crate::context::Label;
use crate::module::Module;

pub struct Repos;

impl Module for Repos {
    fn name(&self) -> &str {
        "repos"
    }

    fn cli_name(&self) -> &str {
        "repo"
    }

    fn describe(&self) -> String {
        "\
## repos/local/
Local repositories are stored here as directories. \
Each repository is initialized as a git repository via `git init`. \
This is a generic place to keep repositories. \
Note: GitHub, GitLab, and Gerrit providers are coming soon — \
those may be a better fit for repositories hosted on a remote platform.\
"
        .to_string()
    }

    fn labels(&self) -> Vec<Label> {
        vec![]
    }

    fn template_paths(&self, _root: &Path) -> Vec<String> {
        vec!["repos/local".to_string()]
    }

    fn init(&self, root: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(root.join("repos/local"))?;
        Ok(())
    }

    fn create(&self, root: &Path, name: &str, _params: &HashMap<String, String>) -> anyhow::Result<()> {
        let repo_path = root.join("repos/local").join(name);
        std::fs::create_dir_all(&repo_path)?;
        let status = std::process::Command::new("git")
            .arg("init")
            .current_dir(&repo_path)
            .status()
            .map_err(|e| anyhow::anyhow!("failed to run git: {}", e))?;
        if !status.success() {
            anyhow::bail!("git init failed in {}", repo_path.display());
        }
        Ok(())
    }

    fn context(&self, root: &Path) -> anyhow::Result<Vec<Value>> {
        let base = root.join("repos/local");
        if !base.exists() {
            return Ok(vec![]);
        }

        let mut items = Vec::new();
        for entry in std::fs::read_dir(&base)? {
            let path = entry?.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                items.push(json!({ "name": name }));
            }
        }
        Ok(items)
    }
}
