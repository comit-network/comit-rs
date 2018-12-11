use crate::{
    ethereum_wallet::InMemoryWallet,
    htlc_harness::{new_account, HTLC_TIMEOUT, SECRET},
    parity_client::ParityClient,
};
use comit_node::swap_protocols::rfc003::{
    ethereum::{EtherHtlc, Seconds},
    Secret,
};
use ethereum_support::{
    web3::{transports::EventLoopHandle, types::Address},
    EtherQuantity,
};
use pretty_env_logger;
use std::{sync::Arc, time::Duration};
use tc_web3_client;
use testcontainers::{images::parity_parity::ParityEthereum, Container, Docker};

#[derive(Debug)]
pub struct EtherHarnessParams {
    pub alice_initial_ether: EtherQuantity,
    pub htlc_timeout: Duration,
    pub htlc_secret: [u8; 32],
    pub htlc_eth_value: EtherQuantity,
}

impl Default for EtherHarnessParams {
    fn default() -> Self {
        Self {
            alice_initial_ether: EtherQuantity::from_eth(1.0),
            htlc_eth_value: EtherQuantity::from_eth(0.4),
            htlc_timeout: HTLC_TIMEOUT,
            htlc_secret: SECRET.clone(),
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

    let alice_client = ParityClient::new(
        Arc::new(InMemoryWallet::new(alice_keypair.clone(), 1)),
        web3,
        0,
    );

    alice_client.give_eth_to(alice, params.alice_initial_ether);

    let tx_id = alice_client.deploy_htlc(
        EtherHtlc::new(
            Seconds::from(params.htlc_timeout),
            alice,
            bob,
            Secret::from(params.htlc_secret).hash(),
        ),
        params.htlc_eth_value.wei(),
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
