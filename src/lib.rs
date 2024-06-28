mod environment;
mod error;
mod railway;

pub use error::{Error, Result};

use crate::environment::{DeserializedEnvironment, DeserializedServiceSource};
pub(crate) use crate::railway::{
    project::Project,
    template::{NewService, NewVolume, Template},
    workflow::{Workflow, WorkflowStatus},
    service::Service,
    deployment::{Deployment, DeploymentLog},
    Railway,
};

use serde::Deserialize;
use std::{collections::HashMap, time::Duration, path::PathBuf};
use tokio::task::JoinSet;
use chrono::Utc;
use tracing::{error, info, warn};
use rand::{thread_rng, prelude::*};

pub async fn run(token: String) -> Result<()> {
    let mut templates = Template::list(&token).await?;
    templates.shuffle(&mut thread_rng());
    info!("Templates: {}", templates.len());

    let (first_chunk, third_chunk) = templates.split_at(templates.len() / 2);
    let (first_chunk, second_chunk) = first_chunk.split_at(first_chunk.len() / 2);
    let (third_chunk, fourth_chunk) = third_chunk.split_at(third_chunk.len() / 2);

    let first_chunk = first_chunk.to_vec();
    let second_chunk = second_chunk.to_vec();
    let third_chunk = third_chunk.to_vec();
    let fourth_chunk = fourth_chunk.to_vec();

    let dir = PathBuf::from(format!("./output/crater-run-{}", Utc::now()));
    tokio::fs::create_dir_all(&dir).await?;

    let mut tasks = JoinSet::new();
    tasks.spawn(run_each(dir.clone(), token.clone(), first_chunk));
    // tasks.spawn(run_each(dir.clone(), token.clone(), second_chunk));
    // tasks.spawn(run_each(dir.clone(), token.clone(), third_chunk));
    // tasks.spawn(run_each(dir.clone(), token.clone(), fourth_chunk));

    let mut results = Vec::new();

    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(res) => results.push(res),
            Err(err) => {
                error!("Error from a chunk: {err}");
            }
        }
    }

    let run = results.into_iter().fold(Run::default(), |mut acc, run| {
        acc.total += run.total;
        acc.valid += run.valid;
        acc.errors.extend(run.errors);
        acc
    });

    info!("Run: {run:#?}");

    Ok(())
}

#[derive(Default, Debug)]
struct Run {
    total: u64,
    healthy: u64,
    valid: u64,
    errors: Vec<Box<dyn std::error::Error + Sync + Send>>,
}

async fn run_each(dir: PathBuf, token: String, chunk: Vec<Template>) -> Run {
    let mut run = Run {
        total: 0,
        healthy: 0,
        valid: 0,
        errors: Vec::new(),
    };

    let mut interval = tokio::time::interval(Duration::from_secs(1));

    'outer: for template in chunk {
        interval.tick().await;

        run.total += 1;

        if template.serialized_config().is_null() {
            warn!("No serialized config for {}, skipping it", template.code());
            continue;
        }

        let config =
            match Option::<DeserializedEnvironment>::deserialize(template.serialized_config()) {
                Ok(config) => config,
                Err(err) => {
                    error!(
                        "Unable to deserialize services for template {}: {err}",
                        template.code()
                    );
                    run.errors.push(Box::new(err));
                    continue;
                }
            };

        let mut services = Vec::new();

        let mut any_healthcheck = false;
        for (id, service) in config.as_ref().map_or(&HashMap::new(), |c| c.services()) {
            let mut variables = HashMap::new();
            for (name, variable) in service.variables() {
                if let Some(value) = variable.default_value().clone().filter(|v| !v.is_empty()) {
                    variables.insert(name.clone(), value);
                } else if !variable.is_optional().unwrap_or_default() {
                    warn!("Missing env var {name} for template {}", template.code());
                    continue 'outer;
                }
            }

            let volumes = service
                .volume_mounts()
                .values()
                .map(|volume| NewVolume {
                    mount_path: volume.mount_path().clone(),
                })
                .collect();

            if service
                .deploy()
                .as_ref()
                .and_then(|d| d.healthcheck_path().clone())
                .is_some()
            {
                any_healthcheck = true;
            }

            services.push(NewService {
                id: id.clone(),
                has_domain: service
                    .networking()
                    .as_ref()
                    .map(|n| !n.service_domains().is_empty()),
                healthcheck_path: service
                    .deploy()
                    .as_ref()
                    .and_then(|d| d.healthcheck_path().clone()),
                name: service.name().clone(),
                root_directory: match service.source() {
                    Some(DeserializedServiceSource::Image { .. }) => None,
                    Some(DeserializedServiceSource::Repo { root_directory, .. }) => {
                        root_directory.clone()
                    }
                    None => None,
                },
                service_icon: service.icon().clone(),
                service_name: service.name().clone(),
                start_command: service
                    .deploy()
                    .as_ref()
                    .and_then(|d| d.start_command().clone()),
                tcp_proxy_application_port: match service
                    .networking()
                    .as_ref()
                    .and_then(|n| n.tcp_proxies().keys().next().map(|k| k.parse::<i64>()))
                    .transpose()
                {
                    Ok(p) => p,
                    Err(err) => {
                        error!(
                            "Invalid tcp procy application port for template {}: {err}",
                            template.code()
                        );
                        run.errors.push(Box::new(err));
                        continue 'outer;
                    }
                },
                template: match service.source() {
                    Some(DeserializedServiceSource::Image { image }) => image.clone(),
                    Some(DeserializedServiceSource::Repo { repo, .. }) => repo.clone(),
                    None => service.name().clone(),
                },
                variables,
                volumes,
            });
        }

        info!("Deploying {}", template.code());
        let deployed = match Template::deploy(&token, services, template.code()).await {
            Ok(deployed) => deployed,
            Err(err) => {
                error!("Unable to deploy template {}: {err}", template.code());
                run.errors.push(Box::new(err));
                continue;
            }
        };
        info!("Checking workflow for {}", template.code());

        let status = if let Some(id) = deployed.workflow_id() {
            match Workflow::status(&token, id).await {
                Ok(status) => status,
                Err(err) => {
                    error!(
                        "Discarded Template {} because of error: {err}",
                        template.code()
                    );
                    run.errors.push(Box::new(err));

                    if let Err(err) = Project::delete(&token, deployed.project_id()).await {
                        error!(
                            "Unable to delete project {} for template {}: {err}",
                            deployed.project_id(),
                            template.code()
                        );
                        run.errors.push(Box::new(err));
                    }
                    continue;
                }
            }
        } else {
            error!("No workflow id for {}", template.code());

            if let Err(err) = Project::delete(&token, deployed.project_id()).await {
                error!("Unable to delete project {}: {err}", deployed.project_id());
                run.errors.push(Box::new(err));
            }
            continue;
        };

        if let WorkflowStatus::Error(err) = status {
            error!("Unable to process {}: {err}", template.code());
            run.errors.push(Box::new(Error::Workflow(err)));

            if let Err(err) = Project::delete(&token, deployed.project_id()).await {
                error!("Unable to delete project {}: {err}", deployed.project_id());
                run.errors.push(Box::new(err));
            }
            continue;
        }

        run.valid += 1;

        info!("Waiting for all builds: {}", template.code());
        if let Err(err) = Service::wait_for_all_builds(&token, deployed.project_id()).await {
            error!("Unable to wait for all builds for {}: {err}", template.code());
            run.errors.push(Box::new(err));

            if let Err(err) = Project::delete(&token, deployed.project_id()).await {
                error!("Unable to delete project {}: {err}", deployed.project_id());
                run.errors.push(Box::new(err));
            }
            continue;
        }

        /*
        if dbg!(any_healthcheck) {
            // TODO: check healthcheck
            /*
            let healthcheck = todo!();
            if healthcheck {
                let full_path = format!("full_path/{healthcheck}");
                reqwest::get(healthcheck)
            }
            */
            tokio::time::sleep(Duration::from_secs(60)).await;
        } else {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
        */

        info!("Listing services");
        let services = match Service::list(&token, deployed.project_id()).await {
            Ok(services) => services,
            Err(err) => {
                run.errors.push(Box::new(err));

                if let Err(err) = Project::delete(&token, deployed.project_id()).await {
                    error!("Unable to delete project {}: {err}", deployed.project_id());
                    run.errors.push(Box::new(err));
                }
                continue;
            }
        };

        for service in &services {
            for instance in service.instances() {
                if let Some(deployment_id) = instance.deployment_id() {
                    let build_logs = match Deployment::build_logs(&token, deployment_id).await {
                        Ok(logs) => logs,
                        Err(err) => {
                            error!("Unable to fetch build logs: {err}");
                            run.errors.push(Box::new(err));

                            if let Err(err) = Project::delete(&token, deployed.project_id()).await {
                                error!("Unable to delete project {}: {err}", deployed.project_id());
                                run.errors.push(Box::new(err));
                            }
                            continue;
                        }
                    };
                    dbg!(&build_logs);

                    let json = match serde_json::to_string(&build_logs) {
                        Ok(json) => json,
                        Err(err) => {
                            error!("Unable to serialize build logs: {err}");
                            run.errors.push(Box::new(err));

                            if let Err(err) = Project::delete(&token, deployed.project_id()).await {
                                error!("Unable to delete project {}: {err}", deployed.project_id());
                                run.errors.push(Box::new(err));
                            }
                            continue;
                        }
                    };

                    if let Err(err) = tokio::fs::write(dir.join(format!("{}-{}.json", service.id(), deployment_id)), &json).await {
                        error!("Unable to serialize build logs: {err}");
                        run.errors.push(Box::new(err));

                        if let Err(err) = Project::delete(&token, deployed.project_id()).await {
                            error!("Unable to delete project {}: {err}", deployed.project_id());
                            run.errors.push(Box::new(err));
                        }
                        continue;
                    }

                    // TODO: collect deployment logs
                }
            }
        }

        if let Err(err) = Project::delete(&token, deployed.project_id()).await {
            error!("Unable to delete project {}: {err}", deployed.project_id());
            run.errors.push(Box::new(err));
            continue;
        }

        info!("Processed template: {}", template.code());
    }

    run
}
