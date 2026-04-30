use serde::Serialize;

#[derive(Serialize)]
pub struct DumpOutput {
    pub project: String,
    pub providers: Vec<ProviderDump>,
}

#[derive(Serialize)]
pub struct ProviderDump {
    pub name: String,
    pub modules: Vec<ModuleDump>,
}

#[derive(Serialize)]
pub struct ModuleDump {
    pub name: String,
    pub parameters: Vec<Param>,
    pub items: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}
