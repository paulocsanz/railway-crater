use derive_get::Getters;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Getters, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeserializedServiceNetworking {
    #[serde(default)]
    service_domains: HashMap<String, serde_json::Value>,
    #[serde(default)]
    tcp_proxies: HashMap<String, serde_json::Value>,
}

#[derive(Getters, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeserializedServiceVolumeMount {
    mount_path: String,
}

#[derive(Getters, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeserializedServiceVariable {
    #[serde(default)]
    default_value: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    is_optional: Option<bool>,
    // #[serde(default)]
    // generator: Option<String>,
}

#[derive(Getters, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeserializedServiceDeploy {
    healthcheck_path: Option<String>,
    start_command: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DeserializedServiceSource {
    Image {
        image: String,
    },
    #[serde(rename_all = "camelCase")]
    Repo {
        root_directory: Option<String>,
        repo: String,
        // branch: Option<String>,
    },
}

#[derive(Getters, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeserializedService {
    // #[serde(default)]
    // build: serde_json::Value,
    #[serde(default)]
    deploy: Option<DeserializedServiceDeploy>,

    #[serde(default)]
    icon: Option<String>,
    name: String,

    #[serde(default)]
    networking: Option<DeserializedServiceNetworking>,

    #[serde(default)]
    source: Option<DeserializedServiceSource>,

    #[serde(default)]
    variables: HashMap<String, DeserializedServiceVariable>,
    #[serde(default)]
    volume_mounts: HashMap<String, DeserializedServiceVolumeMount>,
}

#[derive(Getters, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeserializedEnvironment {
    #[serde(default)]
    services: HashMap<String, DeserializedService>,
}
