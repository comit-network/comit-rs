use crate::{
    ethereum_wallet::InMemoryWallet,
    htlc_harness::{new_account, SECRET},
    parity_client::ParityClient,
};
use comit_node::swap_protocols::rfc003::{ethereum::EtherHtlc, Secret, SecretHash, Timestamp};
use ethereum_support::{
    web3::{transports::EventLoopHandle, types::Address},
    EtherQuantity,
};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tc_web3_client;
use testcontainers::{images::parity_parity::ParityEthereum, Container, Docker};

#[derive(Debug, Clone)]
pub struct EtherHarnessParams {
    pub alice_initial_ether: EtherQuantity,
    pub htlc_refund_timestamp: Timestamp,
    pub relative_timelock: Duration,
    pub htlc_secret_hash: SecretHash,
    pub htlc_eth_value: EtherQuantity,
}

impl Default for EtherHarnessParams {
    fn default() -> Self {
        let current_timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("SystemTime::duration_since failed")
            .as_secs() as u32;
        let relative_timelock = 10;
        let htlc_refund_timestamp = Timestamp::from(current_timestamp + relative_timelock);

        Self {
            alice_initial_ether: EtherQuantity::from_eth(1.0),
            htlc_refund_timestamp,
            relative_timelock: Duration::from_secs(relative_timelock as u64),
            htlc_secret_hash: Secret::from_vec(SECRET).unwrap().hash(),
            htlc_eth_value: EtherQuantity::from_eth(0.4),
        }
    }
}

impl EtherHarnessParams {
    pub fn with_secret_hash(self, secret_hash: SecretHash) -> Self {
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

    let alice_client = ParityClient::new(
        Arc::new(InMemoryWallet::new(alice_keypair.clone(), 1)),
        web3,
        0,
    );

    alice_client.give_eth_to(alice, params.alice_initial_ether);

    let tx_id = alice_client.deploy_htlc(
        EtherHtlc::new(
            params.htlc_refund_timestamp,
            alice,
            bob,
            params.htlc_secret_hash,
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
