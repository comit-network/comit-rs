extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate spectral;
extern crate jsonrpc_minihttp_server;

use self::reqwest::{Client as HTTPClient, Error as ResponseError};
use self::serde::de::{Deserialize, DeserializeOwned, Deserializer};
use self::serde::ser::Serialize;

#[derive(Serialize, Debug, Deserialize, PartialEq)]
enum Version {
    #[serde(rename = "1.0")]
    V1,

    #[serde(rename = "2.0")]
    V2,
}

#[derive(Serialize)]
struct Payload<T> where T: Serialize {
    jsonrpc: Version,
    id: String,
    method: String,
    params: T,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Error {
    code: i32,
    message: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Response<R> {
    Successful {
        id: String,
        #[serde(rename = "jsonrpc")]
        version: Version,
        result: R
    },
    Error {
        id: String,
        #[serde(rename = "jsonrpc")]
        version: Version,
        error: Error
    },
}

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

    pub fn call0<R>(&self, id: &str, method: &str) -> Result<Response<R>, ResponseError> where R: DeserializeOwned {
        self.call::<R, Vec<i32>>(id, method, vec![])
    }

    pub fn call1<R, A>(&self, id: &str, method: &str, a: A) -> Result<Response<R>, ResponseError> where A: Serialize, R: DeserializeOwned {
        self.call(id, method, [a])
    }

    pub fn call2<R, A, B>(&self, id: &str, method: &str, a: A, b: B) -> Result<Response<R>, ResponseError> where A: Serialize, B: Serialize, R: DeserializeOwned {
        self.call(id, method, (a, b))
    }

    fn call<R, Params>(&self, id: &str, method: &str, params: Params) -> Result<Response<R>, ResponseError> where Params: Serialize, R: DeserializeOwned {
        let payload = Payload {
            jsonrpc: Version::V1,
            id: id.to_string(),
            method: method.to_string(),
            params,
        };

        self.client
            .post(self.url.as_str())
            .json(&payload)
            .send()
            .and_then(|mut res| res.json::<Response<R>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::spectral::prelude::*;
    use super::reqwest::header::*;

    use super::jsonrpc_minihttp_server::{ServerBuilder};
    use super::jsonrpc_minihttp_server::jsonrpc_core::{IoHandler};
    use super::serde_json::Value;
    use std::thread::*;
    use std::time::Duration;

    #[test]
    fn can_serialize_payload_with_no_params() {
        let payload = Payload {
            jsonrpc: Version::V1,
            id: "test".to_string(),
            method: "test".to_string(),
            params: (),
        };

        let expected_payload = r#"{"jsonrpc":"1.0","id":"test","method":"test","params":null}"#.to_string();

        let serialized_payload = serde_json::to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }

    #[test]
    fn can_deserialize_successful_response_into_generic_type() {
        let result = r#"{
            "jsonrpc": "1.0",
            "id": "test",
            "result": 519521,
            "error": null
        }"#;

        let deserialized_response: Response<i32> = serde_json::from_str(result).unwrap();

        match deserialized_response {
            Response::Successful { id, version, result } => {
                assert_that(&id).is_equal_to("test".to_string());
                assert_that(&result).is_equal_to(519521);
            }
            Response::Error { id, version, error } => {
                panic!("Should not yield error")
            }
        }
    }

    #[test]
    fn can_deserialize_error_response() {
        let result = r#"{
            "id": "test",
            "jsonrpc": "1.0",
            "result": null,
            "error": {
                "code": -123,
                "message": "Something went wrong"
            }
        }"#;

        let deserialized_response: Response<i32> = serde_json::from_str(result).unwrap();

        match deserialized_response {
            Response::Successful { id, version, result } => {
                panic!("Should not yield successful result");
            }
            Response::Error { id, version, error } => {
                assert_that(&id).is_equal_to("test".to_string());
                assert_that(&error.code).is_equal_to(-123);
                assert_that(&error.message).is_equal_to("Something went wrong".to_string());
            }
        }
    }
//
//    #[test]
//    fn can_send_request_to_actual_server() {
//
//        spawn(move|| {
//            let mut io = IoHandler::default();
//            io.add_method("say_hello", |_| {
//                Ok(Value::String("hello".into()))
//            });
//
//            let server = ServerBuilder::new(io)
//                .start_http(&"127.0.0.1:3030".parse().unwrap())
//                .expect("Unable to start RPC server");
//
//            server.wait().unwrap();
//        });
//
//        sleep(Duration::from_secs(1));
//
//        let http_client = HTTPClient::new();
//        let rpc_client = Client::new(http_client, "http://127.0.0.1:3030");
//
//        let response: Response<String> = rpc_client.call0("test", "say_hello").unwrap();
//
//        match response {
//            Response::Successful { id, version, result } => {
//                assert_that(&id).is_equal_to("test".to_string());
//                assert_that(&result).is_equal_to("hello".to_string());
//            }
//            Response::Error { id, version, error } => {
//                println!("{:?}|{:?}|{:?}", id, version, error);
//                panic!("Should not yield error result");
//            }
//        }
//    }

}