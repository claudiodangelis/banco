use std::path::Path;

pub fn read_git_remote_url(repo_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(repo_path.join(".git/config")).ok()?;
    let mut in_origin = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == r#"[remote "origin"]"# {
            in_origin = true;
        } else if trimmed.starts_with('[') {
            in_origin = false;
        } else if in_origin {
            if let Some(url) = trimmed.strip_prefix("url = ") {
                return Some(url.to_string());
            }
        }
    }
    None
}

pub fn normalize_git_url(url: &str) -> String {
    let url = url.trim_end_matches(".git");
    if let Some(rest) = url.strip_prefix("git@") {
        if let Some((host, path)) = rest.split_once(':') {
            return format!("https://{}/{}", host, path);
        }
    }
    url.to_string()
}
