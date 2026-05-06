use serde::Deserialize;

pub struct GitLabClient {
    base_url: String,
    token: Option<String>,
    client: reqwest::blocking::Client,
}

#[derive(Deserialize)]
pub struct Project {
    pub id: u64,
    pub path: String,
    pub path_with_namespace: String,
    pub ssh_url_to_repo: String,
}

#[derive(Deserialize)]
pub struct Board {
    pub id: u64,
    pub name: String,
}

#[derive(Deserialize)]
pub struct BoardList {
    pub label: BoardLabel,
}

#[derive(Deserialize)]
pub struct BoardLabel {
    pub name: String,
}

#[derive(Deserialize)]
pub struct Issue {
    pub iid: u64,
    pub title: String,
    pub description: Option<String>,
}

impl GitLabClient {
    pub fn new(base_url: &str, token: Option<String>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn get(&self, path: &str) -> reqwest::blocking::RequestBuilder {
        let url = format!("{}/api/v4{}", self.base_url, path);
        let req = self.client.get(url);
        match &self.token {
            Some(t) => req.header("PRIVATE-TOKEN", t),
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
            let mut params: Vec<(&str, &str)> =
                vec![("per_page", "100"), ("page", &page_str)];
            params.extend_from_slice(extra_params);
            let resp = self.get(path).query(&params).send()?.error_for_status()?;
            let next: Option<u32> = resp
                .headers()
                .get("X-Next-Page")
                .and_then(|v| v.to_str().ok())
                .filter(|s| !s.is_empty())
                .and_then(|s| s.parse().ok());
            let items: Vec<T> = resp.json()?;
            let done = items.is_empty() || next.is_none();
            all.extend(items);
            if done {
                break;
            }
            page = next.unwrap();
        }
        Ok(all)
    }

    pub fn all_projects(&self) -> anyhow::Result<Vec<Project>> {
        self.get_paged("/projects", &[("membership", "true"), ("min_access_level", "10")])
    }

    pub fn project(&self, namespace_project: &str) -> anyhow::Result<Project> {
        let encoded = namespace_project.replace('/', "%2F");
        let resp = self
            .get(&format!("/projects/{}", encoded))
            .send()?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    pub fn boards(&self, project_id: u64) -> anyhow::Result<Vec<Board>> {
        self.get_paged(&format!("/projects/{}/boards", project_id), &[])
    }

    pub fn board_lists(&self, project_id: u64, board_id: u64) -> anyhow::Result<Vec<BoardList>> {
        self.get_paged(
            &format!("/projects/{}/boards/{}/lists", project_id, board_id),
            &[],
        )
    }

    /// Opened issues that don't belong to any of the given board list labels ("Open" column).
    pub fn issues_opened(&self, project_id: u64, excluded_labels: &str) -> anyhow::Result<Vec<Issue>> {
        let mut params: Vec<(&str, &str)> = vec![("state", "opened"), ("scope", "all")];
        if !excluded_labels.is_empty() {
            params.push(("not[labels]", excluded_labels));
        }
        self.get_paged(&format!("/projects/{}/issues", project_id), &params)
    }

    /// Opened issues with a specific label (label-based board column).
    pub fn issues_by_label(&self, project_id: u64, label: &str) -> anyhow::Result<Vec<Issue>> {
        self.get_paged(
            &format!("/projects/{}/issues", project_id),
            &[("state", "opened"), ("labels", label), ("scope", "all")],
        )
    }

    /// All closed issues ("Closed" column).
    pub fn issues_closed(&self, project_id: u64) -> anyhow::Result<Vec<Issue>> {
        self.get_paged(
            &format!("/projects/{}/issues", project_id),
            &[("state", "closed"), ("scope", "all")],
        )
    }
}
