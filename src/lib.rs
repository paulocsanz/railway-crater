mod railway;
mod error;
mod environment;

pub use error::{Error, Result};

use crate::railway::{Railway, template::{Template, NewService, NewVolume}};
use crate::environment::{DeserializedEnvironment, DeserializedServiceSource};

use std::{collections::HashMap, time::Duration};
use serde::Deserialize;
use tracing::{error, info, warn, debug};
use tokio::task::JoinSet;

pub async fn run(token: String) -> Result<()> {
    let templates = Template::list(&token).await?;
    info!("Templates: {}", templates.len());

    let (first_chunk, third_chunk) = templates.split_at(templates.len() / 2);
    let (first_chunk, second_chunk) = first_chunk.split_at(first_chunk.len() / 2);
    let (third_chunk, fourth_chunk) = third_chunk.split_at(third_chunk.len() / 2);

    let first_chunk = first_chunk.to_vec();
    let second_chunk = second_chunk.to_vec();
    let third_chunk = third_chunk.to_vec();
    let fourth_chunk = fourth_chunk.to_vec();

    let mut tasks = JoinSet::new();
    tasks.spawn(run_each(token.clone(), first_chunk));
    // tasks.spawn(run_each(token.clone(), second_chunk));
    // tasks.spawn(run_each(token.clone(), third_chunk));
    // tasks.spawn(run_each(token.clone(), fourth_chunk));

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
    valid: u64,
    errors: Vec<Box<dyn std::error::Error + Sync + Send>>,
}

async fn run_each(token: String, chunk: Vec<Template>) -> Run {
    let mut run = Run {
        total: 0,
        valid: 0,
        errors: Vec::new(),
    };

    'outer: for template in chunk {
        run.total += 1;

        let config = match Option::<DeserializedEnvironment>::deserialize(template.serialized_config()) {
            Ok(config) => config,
            Err(err) => {
                run.errors.push(Box::new(err));
                debug!("Discarded Template: {}", template.code());
                continue;
            }
        };

        let mut services = Vec::new();

        for (id, service) in config.as_ref().map_or(&HashMap::new(), |c| c.services()) {
            let mut variables = HashMap::new();
            for (name, variable) in service.variables() {
                if let Some(value) = variable.default_value().clone().filter(|v| !v.is_empty()) {
                    variables.insert(name.clone(), value);
                } else if !variable.is_optional().unwrap_or_default() {
                    // Can't crater run templates with env vars to fill out
                    // warn!("Required empty var {name}");
                    debug!("Discarded Template: {}", template.code());
                    continue 'outer;
                }
            }

            let volumes = service.volume_mounts().values().map(|volume| NewVolume {
                mount_path: volume.mount_path().clone(),
            }).collect();

            services.push(NewService {
                id: id.clone(),
                has_domain: service.networking().as_ref().map(|n| !n.service_domains().is_empty()),
                healthcheck_path: service.deploy().as_ref().and_then(|d| d.healthcheck_path().clone()),
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
                start_command: service.deploy().as_ref().and_then(|d| d.start_command().clone()),
                tcp_proxy_application_port: match service
                    .networking()
                    .as_ref()
                    .and_then(|n| n.tcp_proxies().keys().next().map(|k| k.parse::<i64>()))
                    .transpose() {
                        Ok(p) => p,
                        Err(err) => {
                            run.errors.push(Box::new(err));
                            debug!("Discarded Template: {}", template.code());
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

        let deployed = match Template::deploy(&token, services, template.code()).await {
            Ok(deployed) => deployed,
            Err(err) => {
                error!("Discarded Template {} because of error: {err}", template.code());
                run.errors.push(Box::new(err));
                tokio::time::sleep(Duration::from_secs(30)).await;
                continue;
            }
        };
        let project_id = deployed.project_id;

        run.valid += 1;
        tokio::time::sleep(Duration::from_secs(30)).await;
        info!("Processed template: {}", template.code());
    }
    run
}
