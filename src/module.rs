use std::collections::HashMap;
use std::path::Path;

use crate::context::Param;

pub trait Module {
    fn name(&self) -> &str;
    fn cli_name(&self) -> &str;
    fn describe(&self) -> String;
    fn parameters(&self) -> Vec<Param>;
    fn template_paths(&self, root: &Path) -> Vec<String>;
    fn init(&self, root: &Path) -> anyhow::Result<()>;
    fn create(&self, root: &Path, name: &str, params: &HashMap<String, String>) -> anyhow::Result<()>;
    fn context(&self, root: &Path) -> anyhow::Result<Vec<serde_json::Value>>;
}
