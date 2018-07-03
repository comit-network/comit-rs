extern crate testcontainers;
extern crate web3;

use testcontainers::clients::DockerCli;
use testcontainers::images::{GanacheCli, GanacheCliArgs};
use testcontainers::{Container, Docker, Image, RunArgs};

use web3::Web3;
use web3::transports::{EventLoopHandle, Http};

pub struct GanacheCliNode {
    container_id: String,
    docker: DockerCli,
    _event_loop_handle: EventLoopHandle,
    web3: Web3<Http>,
}

impl GanacheCliNode {
    pub fn new() -> Self {
        let docker = DockerCli {};

        let args = GanacheCliArgs {
            network_id: 42,
            number_of_accounts: 7,
            mnemonic: String::from("supersecure"),
        };

        let ganache_cli = GanacheCli::latest().with_args(args);

        let container_id = docker.run_detached(
            &ganache_cli,
            RunArgs {
                ports: ganache_cli.exposed_ports(),
                rm: true,
                ..RunArgs::default()
            },
        );
        let info = docker.inspect(&container_id);

        let external_port = info.ports().map_to_external_port(8545).unwrap();

        let url = format!("http://localhost:{}", external_port);
        println!("Ganache URL is {}", url);
        let (_event_loop_handle, transport) = Http::new(&url).unwrap();
        let web3 = web3::Web3::new(transport);

        GanacheCliNode {
            container_id,
            docker,
            _event_loop_handle,
            web3,
        }
    }

    pub fn get_client(&self) -> &Web3<Http> {
        &self.web3
    }
}

impl Drop for GanacheCliNode {
    fn drop(&mut self) {
        self.docker.rm(&self.container_id);
    }
}
