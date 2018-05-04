use reqwest::{Client as HTTPClient, Error};
use serde::de::DeserializeOwned;
use serde::Serialize;
use request::RpcRequest;
use response::RpcResponse;

pub struct RpcClient {
    client: HTTPClient,
    url: String,
}

impl RpcClient {
    pub fn new(client: HTTPClient, url: &str) -> Self {
        RpcClient {
            client,
            url: url.to_string(),
        }
    }

    pub fn send<R, T>(&self, request: RpcRequest<T>) -> Result<RpcResponse<R>, Error>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        self.client
            .post(self.url.as_str())
            .json(&request)
            .send()
            .and_then(|mut res| res.json::<RpcResponse<R>>())

        // TODO: Maybe check if req.id == res.id. Should always hold since it is a sychnronous call.
    }
}
