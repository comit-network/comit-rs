extern crate env_logger;
extern crate ganache_rust_web3;
extern crate testcontainers;
extern crate trufflesuite_ganachecli;
extern crate web3;

use ganache_rust_web3::Ganache;
use testcontainers::{clients::DockerCli, Docker};
use trufflesuite_ganachecli::{GanacheCli, Web3Client};
use web3::{futures::Future, transports};

#[test]
fn test_evm_snapshot() {
    let _ = env_logger::try_init();

    let container = DockerCli::new().run(GanacheCli::default());
    let client = container.connect::<Web3Client>();

    let _ = client
        .api::<Ganache<transports::Http>>()
        .evm_snapshot()
        .wait()
        .unwrap();
}

#[test]
fn test_evm_revert() {
    let _ = env_logger::try_init();

    let container = DockerCli::new().run(GanacheCli::default());
    let client = container.connect::<Web3Client>();

    let snapshot_id = client
        .api::<Ganache<transports::Http>>()
        .evm_snapshot()
        .wait()
        .unwrap();

    let _ = client
        .api::<Ganache<transports::Http>>()
        .evm_revert(&snapshot_id)
        .wait()
        .unwrap();
}

#[test]
fn test_evm_increase_time() {
    let _ = env_logger::try_init();

    let container = DockerCli::new().run(GanacheCli::default());

    let client = container.connect::<Web3Client>();

    //        let increase = U256::from(1);
    //
    let _ = client
        .api::<Ganache<transports::Http>>()
        .evm_increase_time(0)
        .wait()
        .unwrap();

    let _ = client
        .api::<Ganache<transports::Http>>()
        .evm_mine()
        .wait()
        .unwrap();
}

#[test]
fn test_evm_mine() {
    let _ = env_logger::try_init();

    let container = DockerCli::new().run(GanacheCli::default());
    let client = container.connect::<Web3Client>();

    let _ = client
        .api::<Ganache<transports::Http>>()
        .evm_mine()
        .wait()
        .unwrap();
}
