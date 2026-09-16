mod client;
mod repos;

use std::path::{Path, PathBuf};

use regex::Regex;

use crate::config::ProviderEntry;
use crate::context::ModuleContext;
use crate::module::{BrowseItem, Module};
use crate::provider::{ConfigParam, ConfigParamKind, Provider, SyncOpts};
use crate::sync_state;

use client::GerritClient;
use repos::GerritRepos;

pub const DEFAULT_SSH_PORT: u16 = 29418;

pub struct GerritProvider {
    entry: ProviderEntry,
}

impl GerritProvider {
    pub fn new(entry: ProviderEntry) -> Self {
        Self { entry }
    }

    pub fn available_modules() -> Vec<&'static str> {
        vec!["repos"]
    }

    pub fn available_config_schema() -> Vec<ConfigParam> {
        vec![
            ConfigParam {
                name: "host",
                description: "Gerrit instance URL (e.g. https://review.leboncoin.ci)",
                kind: ConfigParamKind::String,
                required: true,
            },
            ConfigParam {
                name: "username",
                description: "Gerrit/LDAP username (firstname.lastname), used for the SSH clone URL",
                kind: ConfigParamKind::String,
                required: true,
            },
            ConfigParam {
                name: "ssh_port",
                description: "Gerrit SSH port (default: 29418)",
                kind: ConfigParamKind::String,
                required: false,
            },
            ConfigParam {
                name: "projects",
                description: "Explicit project paths to sync (e.g. leboncoin/go)",
                kind: ConfigParamKind::List,
                required: true,
            },
            ConfigParam {
                name: "namespaced",
                description: "Clone into the full project path (owner/repo) instead of a flat leaf dir — avoids collisions when scopes share a leaf name (default: false)",
                kind: ConfigParamKind::Bool,
                required: false,
            },
        ]
    }

    fn host(&self) -> String {
        self.entry
            .get_str("host")
            .unwrap_or_default()
            .trim_end_matches('/')
            .to_string()
    }

    fn ssh_port(&self) -> u16 {
        self.entry
            .get_str("ssh_port")
            .and_then(|p| p.parse().ok())
            .unwrap_or(DEFAULT_SSH_PORT)
    }

    /// `ssh://user@host:port` base for cloning, derived from `host`, `username`
    /// and `ssh_port`. The scheme and any path are stripped from `host`.
    fn ssh_base(&self) -> String {
        let host = self.host();
        let hostname = host
            .strip_prefix("https://")
            .or_else(|| host.strip_prefix("http://"))
            .unwrap_or(&host);
        let hostname = hostname.split('/').next().unwrap_or(hostname);
        let user = self.entry.get_str("username").unwrap_or_default();
        format!("ssh://{}@{}:{}", user, hostname, self.ssh_port())
    }

    fn repos_root(&self, root: &Path) -> PathBuf {
        root.join("repos").join(self.entry.display_name())
    }

    fn gerrit_repos(&self, root: &Path) -> GerritRepos {
        GerritRepos {
            repos_root: self.repos_root(root),
            ssh_base: self.ssh_base(),
            namespaced: self.entry.get_bool("namespaced"),
        }
    }

    fn client(&self) -> GerritClient {
        GerritClient::new(&self.host(), self.entry.get_str("username"))
    }

    fn resolved_projects(&self) -> anyhow::Result<Vec<String>> {
        let explicit = self.entry.get_list("projects");
        if explicit.is_empty() {
            anyhow::bail!("`projects` must be set");
        }
        Ok(explicit)
    }

    /// Install Gerrit's standard `commit-msg` hook into every cloned repo that
    /// lacks it. The hook stamps the `Change-Id` footer Gerrit requires on push;
    /// without it a first `git push HEAD:refs/for/…` is rejected. Idempotent, and
    /// non-fatal: a failed fetch warns but never fails the sync.
    fn install_commit_msg_hooks(&self, root: &Path) -> anyhow::Result<()> {
        let repos_root = self.repos_root(root);
        let missing: Vec<PathBuf> = crate::providers::git::discover_repos(&repos_root)
            .into_iter()
            .filter(|repo| !repo.join(".git/hooks/commit-msg").exists())
            .collect();
        if missing.is_empty() {
            return Ok(());
        }

        let hook = match self.client().commit_msg_hook() {
            Ok(h) => h,
            Err(e) => {
                eprintln!("  warning: could not fetch commit-msg hook: {}", e);
                return Ok(());
            }
        };

        let mut installed = 0usize;
        for repo in &missing {
            let hooks_dir = repo.join(".git/hooks");
            if let Err(e) = std::fs::create_dir_all(&hooks_dir) {
                eprintln!("  warning: {}: {}", repo.display(), e);
                continue;
            }
            let dest = hooks_dir.join("commit-msg");
            if let Err(e) = std::fs::write(&dest, &hook) {
                eprintln!("  warning: {}: {}", repo.display(), e);
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
            }
            installed += 1;
        }
        if installed > 0 {
            println!("  commit-msg hook installed in {} repo(s)", installed);
        }
        Ok(())
    }
}

/// Extract the Gerrit project path from a clone URL like
/// `ssh://user@host:29418/core/infra/x` → `core/infra/x`. Returns `None` if the
/// URL has no path component.
fn gerrit_project_from_ssh_url(url: &str) -> Option<String> {
    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let (_authority, path) = after_scheme.split_once('/')?;
    let path = path.trim_end_matches('/').trim_end_matches(".git");
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

impl Provider for GerritProvider {
    fn name(&self) -> &str {
        self.entry.display_name()
    }

    fn init(&self, root: &Path) -> anyhow::Result<()> {
        if self.entry.is_module_enabled("repos") {
            std::fs::create_dir_all(self.repos_root(root))?;
        }
        Ok(())
    }

    fn sync(&self, root: &Path, opts: &SyncOpts) -> anyhow::Result<()> {
        let sync_repos = opts.module.as_deref().map_or(true, |m| m == "repos")
            && self.entry.is_module_enabled("repos");
        if !sync_repos {
            return Ok(());
        }

        let mut projects = self.resolved_projects()?;
        if let Some(ref pat) = opts.pattern {
            let re = Regex::new(&format!("^(?:{})$", pat))
                .map_err(|e| anyhow::anyhow!("invalid --pattern: {}", e))?;
            projects.retain(|p| re.is_match(p));
        }

        let synced_at = sync_state::now();
        if !projects.is_empty() {
            self.gerrit_repos(root).sync(&projects, opts.jobs)?;
            self.install_commit_msg_hooks(root)?;
        }
        sync_state::write(root, self.entry.display_name(), &synced_at)?;
        Ok(())
    }

    fn browse_modules(&self, root: &Path) -> anyhow::Result<Vec<(String, Vec<BrowseItem>)>> {
        let host = self.host();
        let repos_base = self.repos_root(root);
        if !self.entry.is_module_enabled("repos") || !repos_base.exists() {
            return Ok(vec![]);
        }

        let mut items: Vec<BrowseItem> = Vec::new();
        for path in crate::providers::git::discover_repos(&repos_base) {
            let display = crate::providers::git::repo_rel_name(&repos_base, &path);
            // The full Gerrit project path (`core/infra/x`) drives the URLs. Under
            // the flat layout the directory name is only the leaf, so recover it
            // from the clone URL; fall back to the directory name otherwise.
            let project = crate::providers::git::read_git_remote_url(&path)
                .and_then(|url| gerrit_project_from_ssh_url(&url))
                .unwrap_or_else(|| display.clone());
            items.push(BrowseItem {
                display,
                pages: vec![
                    ("Repository".to_string(), format!("{}/admin/repos/{}", host, project)),
                    ("Open reviews".to_string(), format!("{}/q/project:{}+status:open", host, project)),
                ],
            });
        }

        if items.is_empty() {
            Ok(vec![])
        } else {
            Ok(vec![("repos".to_string(), items)])
        }
    }

    fn context(&self, root: &Path) -> anyhow::Result<Vec<ModuleContext>> {
        let mut modules = Vec::new();
        if self.entry.is_module_enabled("repos") {
            let repo_items = self.gerrit_repos(root).context(root)?;
            modules.push(ModuleContext {
                name: "repos".to_string(),
                labels: vec![],
                items: repo_items,
            });
        }
        Ok(modules)
    }
}
