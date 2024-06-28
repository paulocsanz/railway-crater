use chrono::{DateTime, Utc};
use std::num::{ParseFloatError, ParseIntError};
use tracing::*;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[remain::sorted]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("date out of range: {0} - {1}")]
    DateOutOfRange(DateTime<Utc>, i64),
    #[error("date truncation")]
    DateTruncation,
    #[error(transparent)]
    DotEnv(#[from] dotenv::Error),
    #[error(transparent)]
    HMacInvalidLength(#[from] hmac::digest::InvalidLength),
    #[error("invalid time delta: secs = {0}, nano = {1}")]
    InvalidTimeDelta(i64, i64),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("json error: {0} with payload {1:#?}")]
    JsonWithMetadata(serde_json::Error, serde_json::Value),
    #[error("missing env var: {0}")]
    MissingEnvVar(&'static str),
    #[error("parse int error for {1}: {0}")]
    ParseFloatWithMetadata(ParseFloatError, String),
    #[error("parse int error for {1}: {0}")]
    ParseIntWithMetadata(ParseIntError, String),
    #[error("railway responded with: {0:?}")]
    Railway(Vec<String>),
    #[error("railway reqwest body error for {1}: {0} ({2:#?})")]
    RailwayBody(reqwest::Error, &'static str, serde_json::Value),
    #[error("railway data missing: {0}")]
    RailwayDataMissing(&'static str),
    #[error("railway reqwest failure for {1}: {0} ({2:#?})")]
    RailwayFailure(reqwest::Error, &'static str, serde_json::Value),
    #[error("railway request failed with status {0}: {1}")]
    RailwayStatusFailure(u16, String),
    #[error("railway reqwest body error for {1}: {0}")]
    WebHookBody(reqwest::Error, String),
    #[error("webhook reqwest failure for {1}: {0}")]
    WebHookFailure(reqwest::Error, String),
    #[error("webhook request failed with status {0}: {1}")]
    WebHookStatusFailure(u16, String),
    #[error("{0}")]
    Workflow(String),
}
