use anyhow::Context;
use futures::TryFutureExt;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct Client {
    inner: reqwest::Client,
    url: url::Url,
}

impl Client {
    pub fn new(base_url: url::Url) -> Self {
        Self {
            inner: reqwest::Client::new(),
            url: base_url,
        }
    }

    pub async fn send<Req, Res>(&self, request: Request<Req>) -> anyhow::Result<Res>
    where
        Req: Debug + Serialize,
        Res: Debug + DeserializeOwned,
    {
        self.send_with_path("".into(), request).await
    }

    pub async fn send_with_path<Req, Res>(
        &self,
        path: String,
        request: Request<Req>,
    ) -> anyhow::Result<Res>
    where
        Req: Debug + Serialize,
        Res: Debug + DeserializeOwned,
    {
        let url = self.url.clone().join(&path)?;

        let response = self
            .inner
            .post(url.clone())
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
    pub fn new(method: &str, params: T, jsonrpc: String) -> Self {
        Self {
            id: "1".to_owned(),
            jsonrpc,
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

#[derive(Debug, serde::Deserialize, thiserror::Error, PartialEq)]
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
