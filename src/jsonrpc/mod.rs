extern crate reqwest;
extern crate serde;
extern crate serde_json;

use self::reqwest::{Client as HTTPClient, Error};
use self::serde::de::Deserialize;
use self::serde::ser::Serialize;
use std::string::String;
use self::serde::Deserializer;

struct JsonRpcClient {
    client: HTTPClient,
    url: String,
}

#[derive(Serialize)]
struct Payload<T> where T: Serialize {
    jsonrpc: String,
    id: String,
    method: String,
    params: T,
}

#[derive(Debug)]
struct Response<'a, R: 'a, E: 'a> where R: Deserialize<'a>, E: Deserialize<'a> {
    id: &'a str,
    result: &'a R,
    error: &'a E,
}


impl<'de: 'a, 'a, R, E> Deserialize<'de> for Response<'a, R, E>
    where R: Deserialize<'a>, E : Deserialize<'a>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where
        D: Deserializer<'de> {
        unimplemented!()
    }
}

impl JsonRpcClient {
    fn new(client: HTTPClient, url: &str) -> Self {
        JsonRpcClient {
            client,
            url: url.to_string(),
        }
    }

    pub fn call0<'a, E, R>(&self, id: &str, method: &str) -> Result<Response<'a, R, E>, Error> where E: Deserialize<'a>, R: Deserialize<'a> {
        self.call::<'a, E, R, Vec<i32>>(id, method, vec![])
    }
//
//    pub fn call1<'a, E, R, A>(&self, id: &str, method: &str, a: A) -> Result<Response<'a, R, E>, Error> where A: Serialize, E: Deserialize<'a>, R: Deserialize<'a> {
//        self.call(id, method, [a])
//    }
//
//    pub fn call2<'a, E, R, A, B>(&self, id: &str, method: &str, a: A, b: B) -> Result<Response<'a, R, E>, Error> where A: Serialize, B: Serialize, E: Deserialize<'a>, R: Deserialize<'a> {
//        self.call(id, method, (a, b))
//    }

    fn call<'a, E, R, Params>(&self, id: &str, method: &str, params: Params) -> Result<Response<'a, R, E>, Error> where Params: Serialize, E: Deserialize<'a>, R: Deserialize<'a> {
        let payload = Payload {
            jsonrpc: "1.0".to_string(),
            id: id.to_string(),
            method: method.to_string(),
            params,
        };

        self.client
            .post(self.url.as_str())
            .json(&payload)
            .send()
            .and_then(|mut res| res.json::<Response<R, E>>())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use super::reqwest::header::*;

    #[test]
    fn test_adssad() {
        let mut headers = Headers::new();
        headers.set(Authorization(Basic {
            username: "bitcoinrpc".to_string(),
            password: Some("ic1RhcJW+aO3G36iAevasRZA+Q0pOJ5GG9uoGrC0DSpo".to_string()),
        }));

        let client = HTTPClient::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        let rpc_client = JsonRpcClient::new(client, "http://127.0.0.1:8332");

//        rpc_client.call1("id", "generate", 100);
        let result: Result<Response<i32, String>, Error> = rpc_client.call0("id", "getblockcount");

        println!("{:?}", result);
    }
}