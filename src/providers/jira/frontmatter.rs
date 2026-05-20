use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct JiraFrontmatter {
    pub status: String,
    #[serde(rename = "type")]
    pub issue_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

pub fn render(fm: &JiraFrontmatter) -> String {
    let yaml = serde_yaml::to_string(fm).unwrap_or_default();
    format!("---\n{}---\n\n", yaml)
}

pub fn parse(content: &str) -> (Option<JiraFrontmatter>, String) {
    let Some(rest) = content.strip_prefix("---\n") else {
        return (None, content.to_string());
    };
    let Some(end) = rest.find("\n---\n") else {
        return (None, content.to_string());
    };
    let fm = serde_yaml::from_str(&rest[..end]).ok();
    let body = rest[end + 5..].strip_prefix('\n').unwrap_or(&rest[end + 5..]).to_string();
    (fm, body)
}

pub fn apply(path: &Path, status: &str, issue_type: &str, parent_id: Option<&str>) -> anyhow::Result<bool> {
    let content = std::fs::read_to_string(path)?;
    let (existing, body) = parse(&content);
    let new_fm = JiraFrontmatter {
        status: status.to_string(),
        issue_type: issue_type.to_string(),
        parent_id: parent_id.map(|s| s.to_string()),
    };
    if existing.as_ref() == Some(&new_fm) {
        return Ok(false);
    }
    std::fs::write(path, format!("{}{}", render(&new_fm), body))?;
    Ok(true)
}
