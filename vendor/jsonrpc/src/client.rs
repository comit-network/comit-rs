use request::RpcRequest;
use reqwest::{Client as HTTPClient, Error};
use response::RpcResponse;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

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

    pub fn send<R: Debug, T: Debug>(&self, request: &RpcRequest<T>) -> Result<RpcResponse<R>, Error>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        debug!("Request: {:?}", request);
        let res = self.client
            .post(self.url.as_str())
            .json(request)
            .send()
            .and_then(|mut res| res.json::<RpcResponse<R>>());
        debug!("Response: {:?}", res);
        res

        // TODO: Maybe check if req.id == res.id. Should always hold since it is a synchronous call.
    }
}
