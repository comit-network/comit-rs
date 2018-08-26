extern crate env_logger;
extern crate ganache_rust_web3;
extern crate tc_trufflesuite_ganachecli;
extern crate tc_web3_client;
extern crate testcontainers;
extern crate web3;

use ganache_rust_web3::Ganache;
use tc_trufflesuite_ganachecli::GanacheCli;
use testcontainers::{clients::DockerCli, Docker};
use web3::{futures::Future, transports};

use tc_web3_client::Web3Client;

#[test]
fn evm_snapshot() {
    let _ = env_logger::try_init();

    let container = DockerCli::new().run(GanacheCli::default());
    let client = Web3Client::new(&container);

    let _ = client
        .api::<Ganache<transports::Http>>()
        .evm_snapshot()
        .wait()
        .unwrap();
}

#[test]
fn evm_revert() {
    let _ = env_logger::try_init();

    let container = DockerCli::new().run(GanacheCli::default());
    let client = Web3Client::new(&container);

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
fn evm_increase_time() {
    let _ = env_logger::try_init();

    let container = DockerCli::new().run(GanacheCli::default());

    let client = Web3Client::new(&container);

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
fn evm_mine() {
    let _ = env_logger::try_init();

    let container = DockerCli::new().run(GanacheCli::default());
    let client = Web3Client::new(&container);

    let _ = client
        .api::<Ganache<transports::Http>>()
        .evm_mine()
        .wait()
        .unwrap();
}
