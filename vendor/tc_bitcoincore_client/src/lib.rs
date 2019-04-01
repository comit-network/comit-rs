#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

use bitcoin_rpc_client::BitcoinCoreClient;
use testcontainers::{images::coblox_bitcoincore::BitcoinCore, Container, Docker};

pub fn new<D: Docker>(container: &Container<'_, D, BitcoinCore>) -> BitcoinCoreClient {
	let port = container.get_host_port(18443).unwrap();
	let auth = container.image().auth();

	let endpoint = format!("http://localhost:{}", port);

	BitcoinCoreClient::new(&endpoint, auth.username(), auth.password())
}
