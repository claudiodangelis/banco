use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
    pub status: String,
    pub tags: Vec<String>,
}

pub fn parse(content: &str) -> (Option<Frontmatter>, String) {
    let Some(rest) = content.strip_prefix("---\n") else {
        return (None, content.to_string());
    };
    let Some(end) = rest.find("\n---\n") else {
        return (None, content.to_string());
    };
    let fm = serde_yaml::from_str(&rest[..end]).ok();
    // body starts after "\n---\n"; strip one leading blank line added by render
    let body = rest[end + 5..].strip_prefix('\n').unwrap_or(&rest[end + 5..]).to_string();
    (fm, body)
}

pub fn render(fm: &Frontmatter) -> String {
    let yaml = serde_yaml::to_string(fm).unwrap_or_default();
    format!("---\n{}---\n\n", yaml)
}

// Adds or updates frontmatter on the file at `path`. Returns true if the file was modified.
pub fn apply(path: &Path, status: &str, tags: &[String]) -> anyhow::Result<bool> {
    let content = std::fs::read_to_string(path)?;
    let (existing, body) = parse(&content);
    let new_fm = Frontmatter {
        status: status.to_string(),
        tags: tags.to_vec(),
    };
    if existing.as_ref() == Some(&new_fm) {
        return Ok(false);
    }
    std::fs::write(path, format!("{}{}", render(&new_fm), body))?;
    Ok(true)
}
