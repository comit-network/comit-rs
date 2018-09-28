#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

extern crate bitcoin_rpc_client;
extern crate tc_coblox_bitcoincore;
extern crate testcontainers;

use bitcoin_rpc_client::BitcoinCoreClient;
use tc_coblox_bitcoincore::BitcoinCore;
use testcontainers::{Container, Docker};

pub fn new<D: Docker>(container: &Container<D, BitcoinCore>) -> BitcoinCoreClient {
    let port = container.get_host_port(18443).unwrap();
    let auth = container.image().auth();

    let endpoint = format!("http://localhost:{}", port);

    BitcoinCoreClient::new(&endpoint, auth.username(), auth.password())
}
