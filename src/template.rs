use std::path::Path;

/// Walks from most specific to least specific label path and returns the content
/// of the first TEMPLATE.md found under `.banco/templates/`, or None if none exists.
///
/// Example: find_template(root, "tasks/gitlab", "my-project/development/2-to-do")
/// checks in order:
///   .banco/templates/tasks/gitlab/my-project/development/2-to-do/TEMPLATE.md
///   .banco/templates/tasks/gitlab/my-project/development/TEMPLATE.md
///   .banco/templates/tasks/gitlab/my-project/TEMPLATE.md
///   .banco/templates/tasks/gitlab/TEMPLATE.md
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
