use std::path::Path;

/// Walks from most specific to least specific label path and returns the content
/// of the first TEMPLATE.md found, or None if no template exists.
pub fn find_template(root: &Path, base: &str, label: &str) -> Option<String> {
    let templates_root = root.join(".banco/templates");
    let mut candidates: Vec<String> = Vec::new();

    if !label.is_empty() {
        let parts: Vec<&str> = label.split('/').collect();
        for depth in (1..=parts.len()).rev() {
            candidates.push(format!("{}/{}", base, parts[..depth].join("/")));
        }
    }
    candidates.push(base.to_string());

    for candidate in candidates {
        let path = templates_root.join(&candidate).join("TEMPLATE.md");
        if path.exists() {
            return std::fs::read_to_string(path).ok();
        }
    }
    None
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
