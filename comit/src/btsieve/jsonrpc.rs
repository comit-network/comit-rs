use anyhow::Context;
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

    pub async fn send<Req, Res>(&self, request: Request<Req>) -> anyhow::Result<Res>
    where
        Req: Debug + Serialize,
        Res: DeserializeOwned,
    {
        let url = self.url.clone();
        let response = self
            .inner
            .post(url.clone())
            .json(&request)
            .send()
            .map_err(ConnectionFailed)
            .await?
            .json::<Response<Res>>()
            .await
            .context("failed to deserialize JSON response as JSON-RPC response")?;

        match response {
            Response::Success { result } => Ok(result),
            Response::Error(error) => {
                Err(error).with_context(|| format!("JSON-RPC request {:?} failed", request))
            }
        }
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

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
pub enum Response<T> {
    Success { result: T },
    Error(JsonRpcError),
}

#[derive(Debug, serde::Deserialize, thiserror::Error)]
#[error("JSON-RPC request failed with code {code}: {message}")]
pub struct JsonRpcError {
    code: i64,
    message: String,
}

#[derive(Debug, thiserror::Error)]
#[error("connection error: {0}")]
pub struct ConnectionFailed(#[from] reqwest::Error);

pub fn serialize<T>(t: T) -> anyhow::Result<serde_json::Value>
where
    T: Serialize,
{
    let value = serde_json::to_value(t).context("failed to serialize parameter")?;

    Ok(value)
}
