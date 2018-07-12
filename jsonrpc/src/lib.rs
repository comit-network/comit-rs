#[macro_use]
extern crate log;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate spectral;

mod client;
mod request;
mod response;
mod version;

pub use client::RpcClient;
pub use request::RpcRequest;
pub use reqwest::Client as HTTPClient;
pub use reqwest::ClientBuilder as HTTPClientBuilder;
pub use reqwest::Error as HTTPError;
pub use response::{RpcError, RpcResponse};
pub use version::JsonRpcVersion;

pub mod header {
    pub use reqwest::header::*;
}
