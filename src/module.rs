use std::collections::HashMap;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::context::Label;

pub struct BrowseItem {
    pub display: String,
    pub pages: Vec<(String, String)>,
}

impl BrowseItem {
    pub fn default_page(display: impl Into<String>, url: impl Into<String>) -> Self {
        Self { display: display.into(), pages: vec![("default".to_string(), url.into())] }
    }
}

pub enum ItemKind {
    FileWithExtension(&'static str),
    Directory,
}

pub trait Module {
    fn name(&self) -> &str;
    fn cli_name(&self) -> &str;
    fn describe(&self) -> String;
    fn labels(&self) -> Vec<Label>;
    fn template_paths(&self, root: &Path) -> Vec<String>;
    fn init(&self, root: &Path) -> anyhow::Result<()>;
    fn create(&self, root: &Path, name: &str, params: &HashMap<String, String>) -> anyhow::Result<PathBuf>;
    fn context(&self, root: &Path) -> anyhow::Result<Vec<serde_json::Value>>;

    fn browse_items(&self, _root: &Path) -> anyhow::Result<Vec<BrowseItem>> {
        Ok(vec![])
    }

    fn root_dirs(&self) -> Vec<&str> {
        vec![]
    }

    fn item_kind(&self) -> ItemKind {
        ItemKind::FileWithExtension("md")
    }

    fn item_matches(&self, path: &Path) -> bool {
        match self.item_kind() {
            ItemKind::FileWithExtension(ext) => {
                path.is_file() && path.extension().map_or(false, |e| e == ext)
            }
            ItemKind::Directory => path.is_dir(),
        }
    }

    fn extraneous_paths(&self, root: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let dirs = self.root_dirs();
        if dirs.is_empty() {
            return Ok(vec![]);
        }
        let base = root.join(dirs[0]).join("local");
        if !base.exists() {
            return Ok(vec![]);
        }
        match self.item_kind() {
            ItemKind::FileWithExtension(ext) => Ok(WalkDir::new(&base)
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .map(|e| e.into_path())
                .filter(|p| p.is_file() && p.extension().map_or(true, |e| e != ext))
                .collect()),
            ItemKind::Directory => Ok(std::fs::read_dir(&base)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| !p.is_dir())
                .collect()),
        }
    }
}
