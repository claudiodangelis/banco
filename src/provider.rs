use std::path::Path;

use crate::dump::ModuleDump;

pub trait Provider {
    fn name(&self) -> &str;
    fn init(&self, root: &Path) -> anyhow::Result<()>;
    fn dump(&self, root: &Path) -> anyhow::Result<Vec<ModuleDump>>;
}
