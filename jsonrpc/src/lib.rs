extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate spectral;

mod client;
mod response;
mod request;
mod version;

pub use client::RpcClient;
pub use request::RpcRequest;
pub use response::{RpcError, RpcResponse};
pub use version::JsonRpcVersion;
pub use reqwest::Client as HTTPClient;
pub use reqwest::ClientBuilder as HTTPClientBuilder;
pub use reqwest::Error as HTTPError;

pub mod header {
    pub use reqwest::header::*;
}
