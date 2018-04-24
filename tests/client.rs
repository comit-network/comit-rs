extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate spectral;
extern crate jsonrpc_minihttp_server;
extern crate swap;

use reqwest::{Client as HTTPClient, Error as ResponseError};
use serde::de::{Deserialize, DeserializeOwned, Deserializer};
use serde::ser::Serialize;
use pectral::prelude::*;
use reqwest::header::*;
use jsonrpc_minihttp_server::{ServerBuilder};
use jsonrpc_minihttp_server::jsonrpc_core::{IoHandler};
use serde_json::Value;
use std::thread;
use std::time::Duration;
use swap::jsonrpc::client::Client;
use swap::jsonrpc::client::Response;
use spectral::assert_that;

//#[test]
//fn can_send_request_to_actual_server() {
//
//    thread::spawn(move|| {
//        let mut io = IoHandler::default();
//        io.add_method("say_hello", |_| {
//            Ok(Value::String("hello".into()))
//        });
//
//        let server = ServerBuilder::new(io)
//            .start_http(&"127.0.0.1:3030".parse().unwrap())
//            .expect("Unable to start RPC server");
//
//        server.wait().unwrap();
//    });
//
//    thread::sleep(Duration::from_secs(1));
//
//    let http_client = HTTPClient::new();
//    let rpc_client = Client::new(http_client, "http://127.0.0.1:3030");
//
//    let response: Response<String> = rpc_client.call0("test", "say_hello").unwrap();
//
//    match response {
//        Response::Successful { id, version, result } => {
//            assert_that(&id).is_equal_to("test".to_string());
//            assert_that(&result).is_equal_to("hello".to_string());
//        }
//        Response::Error { id, version, error } => {
//            println!("{:?}|{:?}|{:?}", id, version, error);
//            panic!("Should not yield error result");
//        }
//    }
//}
