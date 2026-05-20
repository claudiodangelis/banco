use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

const STATE_DIR: &str = ".banco/sync-state";
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

fn state_path(root: &Path, provider: &str) -> PathBuf {
    root.join(STATE_DIR).join(provider)
}

pub fn ensure_dir(root: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(root.join(STATE_DIR))?;
    Ok(())
}

pub fn read(root: &Path, provider: &str) -> Option<String> {
    std::fs::read_to_string(state_path(root, provider))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn write(root: &Path, provider: &str, ts: &DateTime<Utc>) -> anyhow::Result<()> {
    std::fs::write(
        state_path(root, provider),
        ts.format(TIMESTAMP_FORMAT).to_string(),
    )?;
    Ok(())
}

pub fn now() -> DateTime<Utc> {
    Utc::now()
}
