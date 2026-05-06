use serde::Deserialize;
use serde_json::Value;

pub struct GitHubClient {
    base_url: String,
    token: Option<String>,
    client: reqwest::blocking::Client,
}

#[derive(Deserialize)]
pub struct Repo {
    pub name: String,
    pub full_name: String,
    pub ssh_url: String,
}

#[derive(Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub pull_request: Option<Value>,
}

impl GitHubClient {
    pub fn new(base_url: &str, token: Option<String>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn get(&self, path: &str) -> reqwest::blocking::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let req = self
            .client
            .get(url)
            .header("User-Agent", "banco")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        match &self.token {
            Some(t) => req.header("Authorization", format!("Bearer {}", t)),
            None => req,
        }
    }

    fn get_paged<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        extra_params: &[(&str, &str)],
    ) -> anyhow::Result<Vec<T>> {
        let mut all = Vec::new();
        let mut page = 1u32;
        loop {
            let page_str = page.to_string();
            let mut params: Vec<(&str, &str)> = vec![("per_page", "100"), ("page", &page_str)];
            params.extend_from_slice(extra_params);
            let resp = self.get(path).query(&params).send()?.error_for_status()?;
            let has_next = resp
                .headers()
                .get("link")
                .and_then(|v| v.to_str().ok())
                .map_or(false, |l| l.contains(r#"rel="next""#));
            let items: Vec<T> = resp.json()?;
            let done = items.is_empty() || !has_next;
            all.extend(items);
            if done {
                break;
            }
            page += 1;
        }
        Ok(all)
    }

    pub fn repo(&self, owner_repo: &str) -> anyhow::Result<Repo> {
        let resp = self
            .get(&format!("/repos/{}", owner_repo))
            .send()?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    pub fn all_repos(&self) -> anyhow::Result<Vec<Repo>> {
        self.get_paged(
            "/user/repos",
            &[("affiliation", "owner,collaborator,organization_member")],
        )
    }

    pub fn issues_open(&self, owner_repo: &str) -> anyhow::Result<Vec<Issue>> {
        self.get_paged(
            &format!("/repos/{}/issues", owner_repo),
            &[("state", "open")],
        )
    }

    pub fn issues_closed(&self, owner_repo: &str) -> anyhow::Result<Vec<Issue>> {
        self.get_paged(
            &format!("/repos/{}/issues", owner_repo),
            &[("state", "closed")],
        )
    }
}
