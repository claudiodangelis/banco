use serde::Serialize;

#[derive(Serialize)]
pub struct ContextOutput {
    pub project: String,
    pub providers: Vec<ProviderContext>,
}

#[derive(Serialize)]
pub struct ProviderContext {
    pub name: String,
    pub modules: Vec<ModuleContext>,
}

#[derive(Serialize)]
pub struct ModuleContext {
    pub name: String,
    pub labels: Vec<Label>,
    pub items: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct Label {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}
