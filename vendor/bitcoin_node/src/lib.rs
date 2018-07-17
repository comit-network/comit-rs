extern crate bitcoin_rpc;
extern crate testcontainers;

use bitcoin_rpc::*;
use std::env::var;
use testcontainers::clients::DockerCli;
use testcontainers::images::{Bitcoind, BitcoindImageArgs};
use testcontainers::*;

pub struct BitcoinNode {
    container_id: String,
    docker: DockerCli,
    client: BitcoinCoreClient,
}

impl BitcoinNode {
    pub fn new() -> Self {
        let docker = DockerCli {};

        let args = BitcoindImageArgs {
            rpc_auth: "bitcoin:cb77f0957de88ff388cf817ddbc7273$9eaa166ace0d94a29c6eceb831a42458e93faeb79f895a7ee4ce03f4343f8f55".to_string(),
            ..BitcoindImageArgs::default()
        };

        let bitcoind = Bitcoind::new("0.16.0").with_args(args);

        let container_id = docker.run_detached(
            &bitcoind,
            RunArgs {
                ports: bitcoind.exposed_ports(),
                ..RunArgs::default()
            },
        );
        let info = docker.inspect(&container_id);

        let external_port = info.ports().map_to_external_port(18443).unwrap();

        let url = format!("http://localhost:{}", external_port);

        let username = "bitcoin";
        let password = "54pLR_f7-G6is32LP-7nbhzZSbJs_2zSATtZV_r05yg=";

        let client = BitcoinCoreClient::new(url.as_str(), username, password);

        BitcoinNode {
            container_id,
            docker,
            client,
        }
    }

    pub fn get_client(&self) -> &BitcoinCoreClient {
        &self.client
    }
}

impl Drop for BitcoinNode {
    fn drop(&mut self) {
        let keep_container = var("KEEP_CONTAINERS_AFTER_TEST")
            .ok()
            .and_then(|var| var.parse().ok())
            .unwrap_or(false);

        match keep_container {
            true => self.docker.stop(&self.container_id),
            false => self.docker.rm(&self.container_id),
        }
    }
}
