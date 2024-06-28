use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use tracing::trace;

pub mod deployment;
pub mod project;
pub mod template;
pub mod service;
pub mod workflow;

#[derive(Serialize, Deserialize, Debug)]
pub struct RailwayError {
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct RailwayResponse<T> {
    #[serde(default)]
    pub data: Option<T>,
    #[serde(default)]
    pub errors: Vec<RailwayError>,
}

pub struct Railway;

impl Railway {
    pub async fn query<T: serde::de::DeserializeOwned + std::fmt::Debug>(
        token: &str,
        json: serde_json::Value,
    ) -> Result<T> {
        trace!("Executing query: {json:#?}");

        let url = "https://backboard.railway.app/graphql/v2";
        let response = reqwest::Client::new()
            .post(url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&json)
            .fetch_mode_no_cors()
            .send()
            .await
            .map_err(|err| Error::RailwayFailure(err, url, json.clone()))?;

        let status = response.status();
        if status != 200 {
            return Err(Error::RailwayStatusFailure(
                status.as_u16(),
                response
                    .text()
                    .await
                    .map_err(|err| Error::RailwayBody(err, url, json))?,
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|err| Error::RailwayBody(err, url, json))?;
        let response = RailwayResponse::<T>::deserialize(&json)
            .map_err(|err| Error::JsonWithMetadata(err, json))?;
        trace!("Output: {response:#?}");

        if !response.errors.is_empty() {
            Err(Error::Railway(
                response.errors.into_iter().map(|e| e.message).collect(),
            ))
        } else if let Some(data) = response.data {
            Ok(data)
        } else {
            Err(Error::RailwayDataMissing("no data returned for: {query}"))
        }
    }
}
