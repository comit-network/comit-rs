use bitcoin_rpc::BitcoinCoreClient;
use std::env::var;
use testcontainers;
use testcontainers::{Container, Docker, Image, RunArgs};

pub fn create_client() -> BitcoinCoreClient {
    let docker = testcontainers::clients::DockerCli {};
    let bitcoind = testcontainers::images::Bitcoind::latest();

    let id = docker.run_detached(
        &bitcoind,
        RunArgs {
            ports: bitcoind.exposed_ports(),
            rm: true,
            ..RunArgs::default()
        },
    );
    let info = docker.inspect(&id);

    let external_port = info.ports().map_to_external_port(18443).unwrap();

    let url = format!("http://localhost:{}", external_port);

    let username = "bitcoin";
    let password = "54pLR_f7-G6is32LP-7nbhzZSbJs_2zSATtZV_r05yg=";

    BitcoinCoreClient::new(url.as_str(), username, password)
}
