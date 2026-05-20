use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct JiraIssue {
    pub id: String,
    pub title: String,
    pub status: String,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub parent_id: Option<String>,
}

pub fn validate(issues: &[JiraIssue]) -> anyhow::Result<()> {
    for (i, issue) in issues.iter().enumerate() {
        if issue.id.is_empty() {
            anyhow::bail!("issue[{}]: `id` is empty", i);
        }
        if issue.title.is_empty() {
            anyhow::bail!("issue[{}]: `title` is empty", i);
        }
        if issue.status.is_empty() {
            anyhow::bail!("issue[{}]: `status` is empty", i);
        }
        if issue.issue_type.is_empty() {
            anyhow::bail!("issue[{}]: `type` is empty", i);
        }
    }
    Ok(())
}
