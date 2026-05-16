use std::path::Path;

use crate::context::ModuleContext;
use crate::module::BrowseItem;

pub struct ConfigParam {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: ConfigParamKind,
    pub required: bool,
}

pub enum ConfigParamKind {
    String,
    List,
    Bool,
}

pub trait Provider {
    fn name(&self) -> &str;
    fn init(&self, root: &Path) -> anyhow::Result<()>;
    fn context(&self, root: &Path) -> anyhow::Result<Vec<ModuleContext>>;

    fn sync(&self, _root: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    fn browse_modules(&self, _root: &Path) -> anyhow::Result<Vec<(String, Vec<BrowseItem>)>> {
        Ok(vec![])
    }
}
