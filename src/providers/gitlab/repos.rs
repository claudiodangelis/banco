use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::context::Label;
use crate::module::Module;

use super::client::GitLabClient;

pub struct GitLabRepos {
    pub repos_root: PathBuf,
}

fn repo_name(namespace_repo: &str) -> &str {
    namespace_repo.rsplit('/').next().unwrap_or(namespace_repo)
}

impl GitLabRepos {
    pub fn sync(&self, client: &GitLabClient, projects: &[String]) -> anyhow::Result<()> {
        for namespace_project in projects {
            let project = client.project(namespace_project)?;
            let dest = self.repos_root.join(repo_name(namespace_project));

            if dest.exists() {
                println!("  fetching {}", namespace_project);
                let status = std::process::Command::new("git")
                    .args(["-C", dest.to_str().unwrap(), "fetch", "--all", "--prune"])
                    .status()?;
                if !status.success() {
                    eprintln!("  warning: git fetch failed for {}", namespace_project);
                }
            } else {
                println!("  cloning {}", namespace_project);
                std::fs::create_dir_all(&self.repos_root)?;
                let status = std::process::Command::new("git")
                    .args(["clone", &project.ssh_url_to_repo, dest.to_str().unwrap()])
                    .status()?;
                if !status.success() {
                    anyhow::bail!("git clone failed for {}", namespace_project);
                }
            }
        }
        Ok(())
    }
}

impl Module for GitLabRepos {
    fn name(&self) -> &str {
        "repos"
    }

    fn cli_name(&self) -> &str {
        "repo"
    }

    fn describe(&self) -> String {
        "## repos/gitlab/\nGitLab repositories synced from configured projects.".to_string()
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

    fn create(&self, _root: &Path, _name: &str, _params: &HashMap<String, String>) -> anyhow::Result<PathBuf> {
        anyhow::bail!("use `banco sync` to add GitLab repositories")
    }

    fn context(&self, _root: &Path) -> anyhow::Result<Vec<Value>> {
        if !self.repos_root.exists() {
            return Ok(vec![]);
        }
        let mut items = Vec::new();
        for entry in std::fs::read_dir(&self.repos_root)? {
            let path = entry?.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                items.push(json!({ "name": name }));
            }
        }
        Ok(items)
    }
}
