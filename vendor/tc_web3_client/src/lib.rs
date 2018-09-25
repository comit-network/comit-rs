#![warn(
    unused_results,
    unused_extern_crates,
    missing_debug_implementations
)]
#![deny(unsafe_code)]

extern crate testcontainers;
extern crate web3;

use web3::{
    api::Web3,
    transports::{EventLoopHandle, Http},
};

use testcontainers::{Container, Docker, Image};

pub fn new<D: Docker, E: Image>(container: &Container<D, E>) -> (EventLoopHandle, Web3<Http>) {
    let port = container.get_host_port(8545).unwrap();
    let endpoint = format!("http://localhost:{}", port);

    let (event_loop, transport) = Http::new(&endpoint).unwrap();
    let web3 = Web3::new(transport);

    (event_loop, web3)
}
