use crate::{Railway, Result};
use derive_get::Getters;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const BUILD_LOGS: &str = include_str!("../graphql/deployment_build_logs.gql");

#[derive(Getters, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentLog {
    attributes: HashMap<String, String>,
    message: String,
    severity: Option<String>,
    timestamp: String,
}

#[derive(Debug, Clone)]
pub struct Deployment;

impl Deployment {
    pub async fn build_logs(token: &str, deployment_id: &str) -> Result<Vec<DeploymentLog>> {
        let response: DeploymentLogResponse = Railway::query(
            token,
            serde_json::json!({
                "query": BUILD_LOGS,
                "variables": {
                    "deploymentId": deployment_id,
                }
            }),
        )
        .await?;

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct DeploymentLogsAttributeResponse {
            key: String,
            value: String,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct DeploymentLogsResponse {
            attributes: Vec<DeploymentLogsAttributeResponse>,
            message: String,
            severity: Option<String>,
            timestamp: String,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct DeploymentLogResponse {
            deployment_logs: Vec<DeploymentLogsResponse>,
        }
        dbg!(&response);

        Ok(todo!())
    }
}
