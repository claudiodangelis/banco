mod bookmarks;
mod notes;
mod repos;
mod tasks;
mod util;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::context::ModuleContext;
use crate::module::{BrowseItem, Module};
use crate::provider::{ConfigParam, Provider};

pub struct CheckFindings {
    pub extraneous_dirs: Vec<PathBuf>,
    pub extraneous_module_paths: Vec<PathBuf>,
}

pub struct LocalProvider {
    modules: Vec<Box<dyn Module>>,
    /// Modules turned off via the local provider's `disabled_modules` config.
    /// They stay in `modules` (so `check` still treats their dirs as owned) but
    /// are skipped for init/context/browse and refused by `find_module`.
    disabled: Vec<String>,
}

impl LocalProvider {
    pub fn new() -> Self {
        Self::with_disabled(Vec::new())
    }

    pub fn with_disabled(disabled: Vec<String>) -> Self {
        Self {
            modules: vec![Box::new(notes::Notes), Box::new(tasks::Tasks), Box::new(bookmarks::Bookmarks), Box::new(repos::Repos)],
            disabled,
        }
    }

    pub fn available_modules() -> Vec<&'static str> {
        vec!["notes", "tasks", "bookmarks", "repos"]
    }

    /// The local provider takes no config parameters today. Returning an empty
    /// schema lets `banco check` validate it uniformly with remote providers —
    /// any key under `local.config` is flagged as extraneous, and the moment a
    /// real parameter is added here it is validated automatically.
    pub fn available_config_schema() -> Vec<ConfigParam> {
        vec![]
    }

    fn is_enabled(&self, module_name: &str) -> bool {
        !self.disabled.iter().any(|m| m == module_name)
    }

    /// Active modules only — the ones not listed in `disabled_modules`.
    fn enabled_modules(&self) -> impl Iterator<Item = &Box<dyn Module>> {
        self.modules.iter().filter(|m| self.is_enabled(m.name()))
    }

    pub fn find_module(&self, cli_name: &str) -> Option<&dyn Module> {
        self.enabled_modules().find(|m| m.cli_name() == cli_name).map(|m| m.as_ref())
    }

    /// CLI names of the currently-enabled modules, for help/error text.
    pub fn enabled_cli_names(&self) -> Vec<String> {
        self.enabled_modules().map(|m| m.cli_name().to_string()).collect()
    }

    pub fn all_template_paths(&self, root: &Path) -> Vec<String> {
        self.enabled_modules().flat_map(|m| m.template_paths(root)).collect()
    }

    pub fn module_descriptions(&self) -> Vec<String> {
        self.enabled_modules().map(|m| m.describe()).collect()
    }

    pub fn check(&self, root: &Path) -> anyhow::Result<CheckFindings> {
        let project_config = crate::config::load(root)?;
        let valid_providers: HashSet<String> = std::iter::once("local".to_string())
            .chain(project_config.providers.iter().map(|e| e.display_name().to_string()))
            .collect();

        let owned: HashSet<&str> = self.modules.iter().flat_map(|m| m.root_dirs()).collect();

        let mut extraneous_dirs = Vec::new();
        for entry in std::fs::read_dir(root)? {
            let path = entry?.path();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if path.is_dir() && !name.starts_with('.') && name != "misc" && !owned.contains(name) {
                extraneous_dirs.push(path);
            }
        }
        extraneous_dirs.sort();

        let mut extraneous_module_paths = Vec::new();
        for module in &self.modules {
            for module_root_name in module.root_dirs() {
                let module_root = root.join(module_root_name);
                if !module_root.exists() {
                    continue;
                }
                for entry in std::fs::read_dir(&module_root)? {
                    let path = entry?.path();
                    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if path.is_file() || (path.is_dir() && !valid_providers.contains(name)) {
                        extraneous_module_paths.push(path);
                    }
                }
            }

            let mut paths = module.extraneous_paths(root)?;
            paths.sort();
            extraneous_module_paths.extend(paths);
        }

        extraneous_module_paths.sort();
        extraneous_module_paths.dedup();

        Ok(CheckFindings { extraneous_dirs, extraneous_module_paths })
    }
}

impl Provider for LocalProvider {
    fn name(&self) -> &str {
        "local"
    }

    fn init(&self, root: &Path) -> anyhow::Result<()> {
        for module in self.enabled_modules() {
            module.init(root)?;
        }
        Ok(())
    }

    fn context(&self, root: &Path) -> anyhow::Result<Vec<ModuleContext>> {
        self.enabled_modules()
            .map(|m| {
                Ok(ModuleContext {
                    name: m.name().to_string(),
                    labels: m.labels(),
                    items: m.context(root)?,
                })
            })
            .collect()
    }

    fn browse_modules(&self, root: &Path) -> anyhow::Result<Vec<(String, Vec<BrowseItem>)>> {
        let mut result = Vec::new();
        for module in self.enabled_modules() {
            let items = module.browse_items(root)?;
            if !items.is_empty() {
                result.push((module.name().to_string(), items));
            }
        }
        Ok(result)
    }
}
