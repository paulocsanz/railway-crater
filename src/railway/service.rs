use derive_get::Getters;
use crate::{Railway, Result, Error};
use serde::{Deserialize, Serialize};

const LIST: &str = include_str!("../graphql/service_list.gql");

#[derive(Getters, Clone, Debug)]
pub struct ServiceInstance {
    healthcheck_path: Option<String>,
    healthcheck_timeout: Option<u64>,
    static_url: Option<String>,
    status: String,
    deployment_id: String,
}

#[derive(Getters, Clone, Debug)]
pub struct Service {
    id: String,
    name: String,
    instances: Vec<ServiceInstance>
}

impl Service {
    pub async fn list(token: &str, project_id: &str) -> Result<Vec<Self>> {
        let response: ServiceList = Railway::query(
            token,
            serde_json::json!({
                "query": LIST,
                "variables": {
                    "id": project_id,
                }
            }),
        )
        .await?;

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServiceEdgeNodeServiceInstancesEdgeNodeLatestDeployment {
            id: String,
            static_url: Option<String>,
            status: String,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServiceEdgeNodeServiceInstancesEdgeNode {
            healthcheck_path: Option<String>,
            healthcheck_timeout: Option<u64>,
            latest_deployment: Option<ServiceListProjectServiceEdgeNodeServiceInstancesEdgeNodeLatestDeployment>
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServiceEdgeNodeServiceInstancesEdge {
            node: ServiceListProjectServiceEdgeNodeServiceInstancesEdgeNode
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServiceEdgeNodeServiceInstances {
            edges: Vec<ServiceListProjectServiceEdgeNodeServiceInstancesEdge>
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServiceEdgeNode {
            id: String,
            name: String,
            service_instances: ServiceListProjectServiceEdgeNodeServiceInstances
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServiceEdge {
            node: ServiceListProjectServiceEdgeNode
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServices {
            edges: Vec<ServiceListProjectServiceEdge>
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProject {
            services: ServiceListProjectServices
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceList {
            project: ServiceListProject
        }

        let mut views = Vec::new();
        for service in response.project.services.edges {
            views.push(Service {
                id: service.node.id,
                name: service.node.name,
                instances: service.node.service_instances.edges.into_iter().map(|i| {
                    let latest_deployment = i.node.latest_deployment.ok_or(Error::RailwayDataMissing("expected latest deployment"))?;
                    Ok::<_, Error>(ServiceInstance {
                        healthcheck_path: i.node.healthcheck_path,
                        healthcheck_timeout: i.node.healthcheck_timeout,
                        static_url: latest_deployment.static_url,
                        status: latest_deployment.status,
                        deployment_id: latest_deployment.id,
                    })
                }).collect::<Result<Vec<_>>>()?
            });
        }
        Ok(views)
    }
}
