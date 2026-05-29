use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub use crate::template::find_template;

pub fn non_md_files(base: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if !base.exists() {
        return Ok(vec![]);
    }
    let extra = WalkDir::new(base)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.is_file() && p.extension().map_or(true, |e| e != "md"))
        .collect();
    Ok(extra)
}

pub fn label_template_paths(base: &str, root: &Path) -> Vec<String> {
    let mut paths = vec![base.to_string()];
    let dir = root.join(base);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut subdirs: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .map(|name| format!("{}/{}", base, name))
            .collect();
        subdirs.sort();
        paths.extend(subdirs);
    }
    paths
}
