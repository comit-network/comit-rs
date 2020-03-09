use anyhow::Context;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug)]
pub struct Client {
    inner: reqwest::Client,
    url: reqwest::Url,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("json-rpc request failed with code {code}: {message}")]
    JsonRpc { code: i64, message: String },
    #[error("connection error")]
    Connection(#[from] reqwest::Error),
}

impl Client {
    pub fn new(base_url: reqwest::Url) -> Self {
        Self {
            inner: reqwest::Client::new(),
            url: base_url,
        }
    }

    pub async fn send<Req, Res>(&self, request: Request<Req>) -> Result<Res, Error>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        let response = self
            .inner
            .post(self.url.clone())
            .json(&request)
            .send()
            .await?
            .json::<Response<Res>>()
            .await?;

        match response {
            Response::Success { result } => Ok(result),
            Response::Error { code, message } => Err(Error::JsonRpc { code, message }),
        }
    }
}

#[derive(serde::Serialize, Debug)]
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
    Error { code: i64, message: String },
}

pub fn serialize<T>(t: T) -> anyhow::Result<serde_json::Value>
where
    T: Serialize,
{
    let value = serde_json::to_value(t).context("failed to serialize parameter")?;

    Ok(value)
}
