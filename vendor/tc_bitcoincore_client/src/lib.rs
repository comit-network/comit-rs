#![warn(missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

use testcontainers::{images::coblox_bitcoincore::BitcoinCore, Container, Docker};

pub fn new<D: Docker>(container: &Container<'_, D, BitcoinCore>) -> bitcoincore_rpc::Client {
    let port = container.get_host_port(18443).unwrap();
    let auth = container.image().auth();

    let endpoint = format!("http://localhost:{}", port);

    bitcoincore_rpc::Client::new(
        endpoint,
        bitcoincore_rpc::Auth::UserPass(auth.username().to_owned(), auth.password().to_owned()),
    )
    .unwrap()
}
