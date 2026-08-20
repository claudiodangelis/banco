use super::schema::JiraIssue;

pub fn fetch_issues(
    project: &str,
    labels: &[String],
    since: Option<&str>,
) -> anyhow::Result<Vec<JiraIssue>> {
    let mut prompt = format!(
        "Using the Atlassian MCP tools, fetch all JIRA issues from project `{}` assigned to the \
         calling user (me). Exclude issues whose status is Done.",
        project
    );

    if let Some(ts) = since {
        prompt.push_str(&format!(
            " Only include issues updated since {}.",
            ts
        ));
    }

    if !labels.is_empty() {
        prompt.push_str(&format!(
            " Filter by the following labels: {}.",
            labels.join(", ")
        ));
    }

    prompt.push_str(
        " When calling the MCP search tool, request ONLY these fields: \
         id, summary, status, issuetype, parent. \
         Return ONLY a JSON array — no explanation, no markdown, no extra text. \
         Each element must have exactly these fields: \
         \"id\" (string, the issue key e.g. ENG-42), \
         \"title\" (string, the issue summary), \
         \"status\" (string, the status name), \
         \"type\" (string, the issue type name), \
         \"parent_id\" (string or null, the parent issue key if any).",
    );

    let output = std::process::Command::new("claude")
        .args([
            "-p",
            &prompt,
            "--output-format",
            "json",
            "--allowedTools",
            "mcp__atlassian__*,Bash",
        ])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "claude CLI not found in PATH — install it from https://claude.ai/code"
                )
            } else {
                anyhow::anyhow!("failed to spawn claude: {}", e)
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "claude exited with status {}\n\
             {}\n\
             If authentication is required, run `claude` interactively first to complete \
             login, then retry `banco sync`.",
            output.status,
            stderr.trim()
        );
    }

    // claude --output-format json wraps the response in:
    // {"type":"result","subtype":"success","result":"[{...}]",...}
    // where `result` is the assistant's text output as a JSON string.
    let stdout = String::from_utf8_lossy(&output.stdout);

    let inner = extract_result(&stdout).unwrap_or_else(|| stdout.to_string());
    let inner = extract_json_array(&inner).ok_or_else(|| {
        anyhow::anyhow!(
            "failed to parse claude output as JSON: no JSON array found\nRaw output:\n{}",
            stdout.trim()
        )
    })?;

    serde_json::from_str(inner).map_err(|e| {
        anyhow::anyhow!(
            "failed to parse claude output as JSON: {}\nRaw output:\n{}",
            e,
            stdout.trim()
        )
    })
}

/// Unwraps the `result` field from claude's --output-format json envelope.
/// Returns the inner string value, or the raw input if unwrapping fails.
fn extract_result(raw: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Envelope {
        result: String,
    }
    serde_json::from_str::<Envelope>(raw).ok().map(|e| e.result)
}

/// Extracts the JSON array from claude's answer, tolerating surrounding prose
/// or markdown code fences that `claude` emits non-deterministically even when
/// asked for raw JSON. Slices from the first `[` to the last `]`.
fn extract_json_array(s: &str) -> Option<&str> {
    let start = s.find('[')?;
    let end = s.rfind(']')?;
    if end < start {
        return None;
    }
    Some(&s[start..=end])
}
