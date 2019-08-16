use crate::{
    ethereum_helper::{tc_web3_client, InMemoryWallet},
    htlc_harness::{new_account, timestamp::Timestamp, SECRET_HASH},
    parity_client::ParityClient,
};
use blockchain_contracts::ethereum::rfc003::EtherHtlc;
use std::sync::Arc;
use testcontainers::{images::parity_parity::ParityEthereum, Container, Docker};
use web3::{
    transports::EventLoopHandle,
    types::{Address, U256},
};

#[derive(Debug, Clone)]
pub struct EtherHarnessParams {
    pub alice_initial_wei: U256,
    pub htlc_refund_timestamp: Timestamp,
    pub htlc_secret_hash: [u8; 32],
    pub htlc_wei_value: U256,
}

impl Default for EtherHarnessParams {
    fn default() -> Self {
        Self {
            alice_initial_wei: U256::from("1000000000000000000"),
            htlc_refund_timestamp: Timestamp::now().plus(10),
            htlc_secret_hash: SECRET_HASH,
            htlc_wei_value: U256::from("0400000000000000000"),
        }
    }
}

impl EtherHarnessParams {
    pub fn with_secret_hash(self, secret_hash: [u8; 32]) -> Self {
        Self {
            htlc_secret_hash: secret_hash,
            ..self
        }
    }
}

pub fn ether_harness<D: Docker>(
    docker: &D,
    params: EtherHarnessParams,
) -> (
    Address,
    Address,
    Address,
    ParityClient,
    EventLoopHandle,
    Container<'_, D, ParityEthereum>,
) {
    let _ = pretty_env_logger::try_init();

    let (alice_keypair, alice) =
        new_account("63be4b0d638d44b5fee5b050ab0beeeae7b68cde3d829a3321f8009cdd76b992");
    let (_, bob) = new_account("f8218ebf6e2626bd1415c18321496e0c5725f0e1d774c7c2eab69f7650ad6e82");

    let container = docker.run(ParityEthereum::default());

    let (event_loop, web3) = tc_web3_client::new(&container);
    let web3 = Arc::new(web3);

    let alice_client = ParityClient::new(Arc::new(InMemoryWallet::new(alice_keypair, 1)), web3, 0);

    alice_client.give_eth_to(alice, params.alice_initial_wei);

    let tx_id = alice_client.deploy_htlc(
        EtherHtlc::new(
            params.htlc_refund_timestamp.into(),
            alice,
            bob,
            params.htlc_secret_hash,
        )
        .into(),
        params.htlc_wei_value,
    );

    (
        alice,
        bob,
        alice_client.get_contract_address(tx_id),
        alice_client,
        event_loop,
        container,
    )
}
