extern crate tc_trufflesuite_ganachecli;
extern crate testcontainers;
extern crate web3;

use std::ops::Deref;
use web3::{
    api::Web3,
    transports::{EventLoopHandle, Http},
};

use testcontainers::{Container, Docker, Image};

pub struct Web3Client {
    _event_loop: EventLoopHandle,
    web3: Web3<Http>,
}

impl Web3Client {
    pub fn new<D: Docker, E: Image>(container: &Container<D, E>) -> Self {
        let port = container.get_host_port(8545).unwrap();
        let endpoint = format!("http://localhost:{}", port);

        let (_event_loop, transport) = Http::new(&endpoint).unwrap();
        let web3 = Web3::new(transport);

        Web3Client { _event_loop, web3 }
    }
}

impl Deref for Web3Client {
    type Target = Web3<Http>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.web3
    }
}
