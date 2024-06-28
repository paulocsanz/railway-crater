use crate::{Railway, Result};
use derive_get::Getters;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::error;

const STATUS: &str = include_str!("../graphql/workflow_status.gql");

#[derive(Debug, Clone)]
pub enum WorkflowStatus {
    Complete,
    Error(String),
}

pub struct Workflow;

impl Workflow {
    pub async fn status(token: &str, id: &str) -> Result<WorkflowStatus> {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            let response: WorkflowStatusResponse = Railway::query(
                token,
                serde_json::json!({
                    "query": STATUS,
                    "variables": {
                        "workflowId": id,
                    }
                }),
            )
            .await?;

            if let Some(err) = response.workflow_status.error.as_deref() {
                error!("Error: {err}");
            }

            let status = match response.workflow_status.status {
                WorkflowStatusEnumResponse::Complete => WorkflowStatus::Complete,
                WorkflowStatusEnumResponse::Error => WorkflowStatus::Error(
                    response.workflow_status.error.clone().unwrap_or_default(),
                ),
                WorkflowStatusEnumResponse::NotFound => {
                    WorkflowStatus::Error("Not Found".to_owned())
                }
                WorkflowStatusEnumResponse::Running => continue,
            };
            return Ok(status);
        }

        #[derive(Serialize, Deserialize, Debug, Copy, Clone)]
        pub enum WorkflowStatusEnumResponse {
            Complete,
            Error,
            NotFound,
            Running,
        }

        #[derive(Getters, Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct WorkflowStatusInternalResponse {
            error: Option<String>,
            status: WorkflowStatusEnumResponse,
        }

        #[derive(Getters, Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct WorkflowStatusResponse {
            workflow_status: WorkflowStatusInternalResponse,
        }
    }
}
