# Testcontainers

Testcontainers is a Rust-library inspired by [http://testcontainers.org](http://testcontainers.org).

It's main purpose is to start, connect and shutdown docker containers from Rust during integration tests.

## Supported docker images

Currently, it supports the following images:

- `ruimarinho/bitcoin-core`
- `trufflesuite/ganache-cli`

## Example Usage

```rust
extern crate testcontainers;

use testcontainers::clients::DockerCli;
use testcontainers::images::{GanacheCli, GanacheCliArgs};
use testcontainers::{Container, Docker, Image, RunArgs};

fn main() {
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
            ..RunArgs::default()
        },
    );
    
    let info = docker.inspect(&container_id);

    let external_port = info.ports().map_to_external_port(8545).unwrap();

    let url = format!("http://localhost:{}", external_port);

    docker.rm(&container_id);
}

```

## Todo

- Create proper Rust docs
- Publish on crates.io
- Support more docker images
- Enrich API