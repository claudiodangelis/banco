use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::context::Label;
use crate::module::Module;

use super::client::GitLabClient;

pub struct GitLabRepos {
    pub repos_root: PathBuf,
    /// When true, clone into the full project path (`repos/gitlab/namespace/project`);
    /// otherwise into a flat leaf directory (`repos/gitlab/project`).
    pub namespaced: bool,
}

fn repo_name(namespace_repo: &str) -> &str {
    namespace_repo.rsplit('/').next().unwrap_or(namespace_repo)
}

impl GitLabRepos {
    pub fn sync(&self, client: &GitLabClient, projects: &[String], jobs: usize) -> anyhow::Result<()> {
        crate::providers::git::sync_repos(projects, &self.repos_root, jobs, |namespace_project| {
            let project = client.project(namespace_project)?;
            let dest = if self.namespaced {
                self.repos_root.join(namespace_project)
            } else {
                self.repos_root.join(repo_name(namespace_project))
            };
            Ok((project.ssh_url_to_repo, dest))
        })
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
        for path in crate::providers::git::discover_repos(&self.repos_root) {
            let name = crate::providers::git::repo_rel_name(&self.repos_root, &path);
            items.push(crate::providers::git::repo_item(name, &path));
        }
        Ok(items)
    }
}
