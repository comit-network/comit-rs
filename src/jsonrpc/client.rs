use reqwest::{Client as HTTPClient, Error as ResponseError};
use jsonrpc::request::Request;
use jsonrpc::response::Response;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Client {
    client: HTTPClient,
    url: String,
}

impl Client {
    pub fn new(client: HTTPClient, url: &str) -> Self {
        Client {
            client,
            url: url.to_string(),
        }
    }

    pub fn send<R, T>(&self, request: Request<T>) -> Result<Response<R>, ResponseError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        self.client
            .post(self.url.as_str())
            .json(&request)
            .send()
            .and_then(|mut res| res.json::<Response<R>>())
    }
}
