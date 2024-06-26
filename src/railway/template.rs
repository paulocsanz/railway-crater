use crate::{Railway, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use derive_get::Getters;

const TEMPLATES: &str = include_str!("../graphql/templates.gql");
const TEMPLATE_DEPLOY: &str = include_str!("../graphql/template_deploy.gql");

#[derive(Getters, Serialize, Debug, Clone)]
pub struct NewVolume {
    pub mount_path: String,
}

#[derive(Getters, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NewService {
    pub id: String,
    #[copy]
    pub has_domain: Option<bool>,
    pub healthcheck_path: Option<String>,
    pub name: String,
    pub service_name: String,
    pub root_directory: Option<String>,
    pub service_icon: Option<String>,
    pub start_command: Option<String>,
    #[copy]
    pub tcp_proxy_application_port: Option<i64>,
    pub template: String,
    pub variables: HashMap<String, String>,
    pub volumes: Vec<NewVolume>,
}

#[derive(Getters, Debug, Clone)]
pub struct Template {
    id: String,
    code: String,
    #[copy]
    health: Option<f64>,
    serialized_config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeployedTemplate {
    project_id: String,
    workflow_id: Option<String>,
}


impl Template {
    pub async fn list(
        token: &str,
    ) -> Result<Vec<Template>> {
        let response: TemplatesResponse = Railway::query(
            token,
            serde_json::json!({
                "query": TEMPLATES,
            }),
        )
        .await?;

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct TemplatesPageEdgeNodeResponse {
            id: String,
            code: String,
            health: Option<f64>,
            serialized_config: serde_json::Value,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct TemplatesPageEdgeResponse {
            cursor: serde_json::Value,
            node: TemplatesPageEdgeNodeResponse,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct TemplatesEdgesResponse {
            edges: Vec<TemplatesPageEdgeResponse>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct TemplatesResponse {
            templates: TemplatesEdgesResponse,
        }

        let mut templates = Vec::with_capacity(response.templates.edges.len());
        for template in response.templates.edges {
            templates.push(Template {
                id: template.node.id,
                code: template.node.code,
                health: template.node.health,
                serialized_config: template.node.serialized_config,
            });
        }

        Ok(templates)
    }

    pub async fn deploy(
        token: &str,
        services: Vec<NewService>,
        template_code: &str
    ) -> Result<DeployedTemplate> {
        let response: DeployedTemplateResponse = Railway::query(
            token,
            serde_json::json!({
                "query": TEMPLATE_DEPLOY,
                "variables": {
                    "services": serde_json::to_value(services)?,
                    "templateCode": template_code,
                }
            }),
        )
        .await?;

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct DeployedTemplateResponse {
            template_deploy: DeployedTemplate,
        }

        Ok(response.template_deploy)
    }
}
