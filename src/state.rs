//! Per-user, per-project UI state.
//!
//! Unlike `.banco/config.yml` — which describes the project and is meant to be
//! committed and shared — this holds personal view preferences (e.g. which TUI
//! provider sections are collapsed). It lives outside the repo under the XDG
//! state directory so it never shows up in a diff and never leaks between users.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// View preferences for a single project, keyed by its canonical path.
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ProjectState {
    /// Names of providers whose dashboard section is collapsed to a summary bar.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collapsed_providers: Vec<String>,
}

impl ProjectState {
    pub fn is_collapsed(&self, provider: &str) -> bool {
        self.collapsed_providers.iter().any(|p| p == provider)
    }

    /// Flip the collapsed state of `provider`, returning the new value.
    pub fn toggle_collapsed(&mut self, provider: &str) -> bool {
        if let Some(pos) = self.collapsed_providers.iter().position(|p| p == provider) {
            self.collapsed_providers.remove(pos);
            false
        } else {
            self.collapsed_providers.push(provider.to_string());
            true
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
struct StateFile {
    #[serde(default)]
    projects: BTreeMap<String, ProjectState>,
}

/// `$XDG_STATE_HOME/banco/state.yml`, falling back to `~/.local/state/banco/state.yml`.
fn state_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(base.join("banco").join("state.yml"))
}

/// Stable key for a project: its canonical path, or the given path if it can't
/// be canonicalized (e.g. it doesn't exist yet).
fn project_key(root: &Path) -> String {
    root.canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

fn load_file() -> StateFile {
    state_path()
        .filter(|p| p.exists())
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_yaml::from_str(&s).ok())
        .unwrap_or_default()
}

/// Load the saved view state for `root`, or defaults if none is stored.
pub fn load(root: &Path) -> ProjectState {
    load_file()
        .projects
        .remove(&project_key(root))
        .unwrap_or_default()
}

/// Persist the view state for `root`. Best-effort: a UI toggle is not worth
/// surfacing an error for, so failures (e.g. unwritable state dir) are ignored.
pub fn save(root: &Path, state: &ProjectState) {
    let Some(path) = state_path() else { return };
    let mut file = load_file();
    file.projects.insert(project_key(root), state.clone());
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    if let Ok(content) = serde_yaml::to_string(&file) {
        let _ = std::fs::write(&path, content);
    }
}
