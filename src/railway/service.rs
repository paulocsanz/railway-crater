use crate::{Error, Railway, Result};
use derive_get::Getters;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const LIST: &str = include_str!("../graphql/service_list.gql");

#[derive(Getters, Clone, Debug)]
pub struct ServiceInstance {
    healthcheck_path: Option<String>,
    healthcheck_timeout: Option<u64>,
    static_url: Option<String>,
    status: Option<String>,
    deployment_id: Option<String>,
}

#[derive(Getters, Clone, Debug)]
pub struct Service {
    id: String,
    name: String,
    instances: Vec<ServiceInstance>,
}

impl Service {
    pub async fn wait_for_all_builds(token: &str, project_id: &str) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        'outer: loop {
            interval.tick().await;

            let services = Self::list(token, project_id).await?;
            for service in services {
                if service.instances().is_empty() {
                    continue;
                }

                for instance in service.instances() {
                    let status = instance.status().as_deref();
                    if status.is_none()
                        || status == Some("BUILDING")
                        || status == Some("WAITING")
                        || status == Some("INITIALIZING")
                        || status == Some("QUEUED")
                    {
                        continue 'outer;
                    }
                }
            }
            break;
        }
        Ok(())
    }

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
            latest_deployment:
                Option<ServiceListProjectServiceEdgeNodeServiceInstancesEdgeNodeLatestDeployment>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServiceEdgeNodeServiceInstancesEdge {
            node: ServiceListProjectServiceEdgeNodeServiceInstancesEdgeNode,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServiceEdgeNodeServiceInstances {
            edges: Vec<ServiceListProjectServiceEdgeNodeServiceInstancesEdge>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServiceEdgeNode {
            id: String,
            name: String,
            service_instances: ServiceListProjectServiceEdgeNodeServiceInstances,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServiceEdge {
            node: ServiceListProjectServiceEdgeNode,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProjectServices {
            edges: Vec<ServiceListProjectServiceEdge>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceListProject {
            services: ServiceListProjectServices,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ServiceList {
            project: ServiceListProject,
        }

        let mut views = Vec::new();
        for service in response.project.services.edges {
            views.push(Service {
                id: service.node.id,
                name: service.node.name,
                instances: service
                    .node
                    .service_instances
                    .edges
                    .into_iter()
                    .map(|i| {
                        Ok::<_, Error>(ServiceInstance {
                            healthcheck_path: i.node.healthcheck_path,
                            healthcheck_timeout: i.node.healthcheck_timeout,
                            static_url: i
                                .node
                                .latest_deployment
                                .as_ref()
                                .and_then(|d| d.static_url.clone()),
                            status: i.node.latest_deployment.as_ref().map(|d| d.status.clone()),
                            deployment_id: i.node.latest_deployment.as_ref().map(|d| d.id.clone()),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            });
        }
        Ok(views)
    }
}
