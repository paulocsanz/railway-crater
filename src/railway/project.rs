use crate::{Error, Railway, Result};
use serde::{Deserialize, Serialize};

const DELETE: &str = include_str!("../graphql/project_delete.gql");

pub struct Project;

impl Project {
    pub async fn delete(token: &str, project_id: &str) -> Result<()> {
        let response: ProjectDeleteResponse = Railway::query(
            dbg!(token),
            dbg!(serde_json::json!({
                "query": DELETE,
                "variables": {
                    "id": project_id,
                }
            })),
        )
        .await?;

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ProjectDeleteResponse {
            project_delete: bool,
        }

        if !response.project_delete {
            return Err(Error::RailwayStatusFailure(
                0,
                format!("Unable to delete project: {project_id}"),
            ));
        }

        Ok(())
    }
}
