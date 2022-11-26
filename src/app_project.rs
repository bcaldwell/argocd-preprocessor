use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppProject {
    pub api_version: String,
    pub kind: String,
    pub metadata: AppProjectMetadata,
    pub spec: AppProjectSpec,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppProjectMetadata {
    pub name: String,
    pub namespace: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppProjectSpec {
    pub destinations: Vec<AppProjectDestination>,
    pub cluster_resource_whitelist: Vec<AppProjectClusterResourceWhitelist>,
    pub source_repos: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppProjectDestination {
    pub name: String,
    pub namespace: String,
    pub server: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppProjectClusterResourceWhitelist {
    pub group: String,
    pub kind: String,
}
