use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::context::Label;
use crate::module::Module;

use super::client::GitHubClient;

pub struct GitHubRepos {
    pub repos_root: PathBuf,
}

impl GitHubRepos {
    pub fn sync(&self, client: &GitHubClient, projects: &[String], jobs: usize) -> anyhow::Result<()> {
        crate::providers::git::sync_repos(projects, &self.repos_root, jobs, |owner_repo| {
            let repo = client.repo(owner_repo)?;
            Ok((repo.ssh_url, self.repos_root.join(&repo.name)))
        })
    }
}

impl Module for GitHubRepos {
    fn name(&self) -> &str {
        "repos"
    }

    fn cli_name(&self) -> &str {
        "repo"
    }

    fn describe(&self) -> String {
        "## repos/github/\nGitHub repositories synced from configured projects.".to_string()
    }

    fn labels(&self) -> Vec<Label> {
        vec![]
    }

    fn template_paths(&self, _root: &Path) -> Vec<String> {
        vec![]
    }

    fn init(&self, _root: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.repos_root)?;
        Ok(())
    }

    fn create(
        &self,
        _root: &Path,
        _name: &str,
        _params: &HashMap<String, String>,
    ) -> anyhow::Result<PathBuf> {
        anyhow::bail!("use `banco sync` to add GitHub repositories")
    }

    fn context(&self, _root: &Path) -> anyhow::Result<Vec<Value>> {
        if !self.repos_root.exists() {
            return Ok(vec![]);
        }
        let mut items = Vec::new();
        for entry in std::fs::read_dir(&self.repos_root)? {
            let path = entry?.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                items.push(crate::providers::git::repo_item(name, &path));
            }
        }
        Ok(items)
    }
}
