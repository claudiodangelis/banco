mod bookmarks;
mod notes;
mod repos;
mod tasks;
mod util;

use std::path::Path;

use crate::context::ModuleContext;
use crate::module::Module;
use crate::provider::Provider;

pub struct LocalProvider {
    modules: Vec<Box<dyn Module>>,
}

impl LocalProvider {
    pub fn new() -> Self {
        Self {
            modules: vec![Box::new(notes::Notes), Box::new(tasks::Tasks), Box::new(bookmarks::Bookmarks), Box::new(repos::Repos)],
        }
    }

    pub fn find_module(&self, cli_name: &str) -> Option<&dyn Module> {
        self.modules.iter().find(|m| m.cli_name() == cli_name).map(|m| m.as_ref())
    }

    pub fn all_template_paths(&self, root: &Path) -> Vec<String> {
        self.modules.iter().flat_map(|m| m.template_paths(root)).collect()
    }

    pub fn module_descriptions(&self) -> Vec<String> {
        self.modules.iter().map(|m| m.describe()).collect()
    }
}

impl Provider for LocalProvider {
    fn name(&self) -> &str {
        "local"
    }

    fn init(&self, root: &Path) -> anyhow::Result<()> {
        for module in &self.modules {
            module.init(root)?;
        }
        Ok(())
    }

    fn context(&self, root: &Path) -> anyhow::Result<Vec<ModuleContext>> {
        self.modules
            .iter()
            .map(|m| {
                Ok(ModuleContext {
                    name: m.name().to_string(),
                    labels: m.labels(),
                    items: m.context(root)?,
                })
            })
            .collect()
    }

    fn browse_modules(&self, root: &Path) -> anyhow::Result<Vec<(String, Vec<(String, String)>)>> {
        let mut result = Vec::new();
        for module in &self.modules {
            let items = module.browse_items(root)?;
            if !items.is_empty() {
                result.push((module.name().to_string(), items));
            }
        }
        Ok(result)
    }
}
