use anyhow::{Context, Result};
use futures::TryFutureExt;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

#[derive(Debug)]
pub struct Client {
    inner: reqwest::Client,
    url: reqwest::Url,
}

impl Client {
    pub fn new(base_url: reqwest::Url) -> Self {
        Self {
            inner: reqwest::Client::new(),
            url: base_url,
        }
    }

    pub async fn send<Req, Res>(&self, request: Request<Req>) -> Result<Res>
    where
        Req: Debug + Serialize,
        Res: DeserializeOwned,
    {
        let response = self
            .inner
            .post(self.url.clone())
            .json(&request)
            .send()
            .map_err(ConnectionFailed)
            .await?
            .json::<Response<Res>>()
            .await
            .context("failed to deserialize JSON response as JSON-RPC response")?
            .payload
            .into_result()
            .with_context(|| {
                format!(
                    "JSON-RPC request {} failed",
                    serde_json::to_string(&request).expect("can always serialize to JSON")
                )
            })?;

        Ok(response)
    }
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct Request<T> {
    id: String,
    jsonrpc: String,
    method: String,
    params: T,
}

impl<T> Request<T> {
    pub fn new(method: &str, params: T) -> Self {
        Self {
            id: "1".to_owned(),
            jsonrpc: "2.0".to_owned(),
            method: method.to_owned(),
            params,
        }
    }
}

#[derive(serde::Deserialize, Debug, PartialEq)]
pub struct Response<R> {
    #[serde(flatten)]
    pub payload: ResponsePayload<R>,
}

#[derive(serde::Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ResponsePayload<R> {
    Result(R),
    Error(JsonRpcError),
}

impl<R> ResponsePayload<R> {
    fn into_result(self) -> Result<R, JsonRpcError> {
        match self {
            ResponsePayload::Result(result) => Ok(result),
            ResponsePayload::Error(e) => Err(e),
        }
    }
}

#[derive(Debug, serde::Deserialize, PartialEq, thiserror::Error)]
#[error("JSON-RPC request failed with code {code}: {message}")]
pub struct JsonRpcError {
    code: i64,
    message: String,
}

#[derive(Debug, thiserror::Error)]
#[error("connection error: {0}")]
pub struct ConnectionFailed(#[from] reqwest::Error);

pub fn serialize<T>(t: T) -> Result<serde_json::Value>
where
    T: Serialize,
{
    let value = serde_json::to_value(t).context("failed to serialize parameter")?;

    Ok(value)
}
