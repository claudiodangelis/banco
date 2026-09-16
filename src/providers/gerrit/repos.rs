use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::context::Label;
use crate::module::Module;

pub struct GerritRepos {
    pub repos_root: PathBuf,
    /// SSH base URL without the project path, e.g. `ssh://user@host:29418`.
    pub ssh_base: String,
    /// When true, clone into the full project path (`repos/gerrit/core/infra/x`);
    /// otherwise into a flat leaf directory (`repos/gerrit/x`).
    pub namespaced: bool,
}

impl GerritRepos {
    pub fn sync(&self, projects: &[String], jobs: usize) -> anyhow::Result<()> {
        crate::providers::git::sync_repos(projects, &self.repos_root, jobs, |project| {
            let url = format!("{}/{}", self.ssh_base, project);
            Ok((url, self.repos_root.join(self.repo_dir(project))))
        })
    }

    /// Directory (relative to `repos_root`) a project clones into: the full path
    /// when namespaced, else the last path segment.
    fn repo_dir<'a>(&self, project: &'a str) -> &'a str {
        if self.namespaced {
            project
        } else {
            project.rsplit('/').next().unwrap_or(project)
        }
    }
}

impl Module for GerritRepos {
    fn name(&self) -> &str {
        "repos"
    }

    fn cli_name(&self) -> &str {
        "repo"
    }

    fn describe(&self) -> String {
        "## repos/gerrit/\nGerrit repositories synced from configured projects.".to_string()
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
        anyhow::bail!("use `banco sync` to add Gerrit repositories")
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
