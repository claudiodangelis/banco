use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::context::Label;

pub trait Module {
    fn name(&self) -> &str;
    fn cli_name(&self) -> &str;
    fn describe(&self) -> String;
    fn labels(&self) -> Vec<Label>;
    fn template_paths(&self, root: &Path) -> Vec<String>;
    fn init(&self, root: &Path) -> anyhow::Result<()>;
    fn create(&self, root: &Path, name: &str, params: &HashMap<String, String>) -> anyhow::Result<PathBuf>;
    fn context(&self, root: &Path) -> anyhow::Result<Vec<serde_json::Value>>;
}
