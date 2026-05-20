use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Scans `dir` and returns a map of issue ID → file path.
/// Filenames are expected to be `{id} - {title}.md`.
pub fn scan(dir: &Path) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    if !dir.exists() {
        return map;
    }
    for entry in std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            if let Some(id) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.splitn(2, " - ").next())
                .map(|s| s.trim().to_string())
            {
                if !id.is_empty() {
                    map.insert(id, path);
                }
            }
        }
    }
    map
}
