use std::path::Path;

pub use crate::template::find_template;

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
