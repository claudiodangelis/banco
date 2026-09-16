/// Minimal Gerrit HTTP client. Used only to fetch the server's `commit-msg` hook
/// so it can be installed into freshly cloned repos; project lists come from the
/// explicit `projects` config, which needs no network access.
pub struct GerritClient {
    base_url: String,
    username: Option<String>,
    client: reqwest::blocking::Client,
}

impl GerritClient {
    pub fn new(base_url: &str, username: Option<String>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            username,
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Fetch the server's standard `commit-msg` hook script. Every Gerrit serves
    /// it at `/tools/hooks/commit-msg` as raw text (no XSSI guard, no auth needed);
    /// installed into a clone it stamps the `Change-Id` footer Gerrit requires.
    pub fn commit_msg_hook(&self) -> anyhow::Result<String> {
        let url = format!("{}/tools/hooks/commit-msg", self.base_url);
        let mut req = self.client.get(url);
        if let Some(user) = &self.username {
            req = req.basic_auth(user, None::<String>);
        }
        Ok(req.send()?.error_for_status()?.text()?)
    }
}
