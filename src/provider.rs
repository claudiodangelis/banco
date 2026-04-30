use std::path::Path;

use crate::context::ModuleContext;

pub trait Provider {
    fn name(&self) -> &str;
    fn init(&self, root: &Path) -> anyhow::Result<()>;
    fn context(&self, root: &Path) -> anyhow::Result<Vec<ModuleContext>>;
}
