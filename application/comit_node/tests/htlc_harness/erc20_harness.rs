use crate::{
    ethereum_wallet::InMemoryWallet,
    htlc_harness::{new_account, SECRET},
    parity_client::ParityClient,
};
use comit_node::swap_protocols::rfc003::{ethereum::Erc20Htlc, Secret, SecretHash, Timestamp};
use ethereum_support::{
    web3::{
        transports::EventLoopHandle,
        types::{Address, U256},
    },
    Erc20Quantity, EtherQuantity,
};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tc_web3_client;
use testcontainers::{images::parity_parity::ParityEthereum, Container, Docker};

#[derive(Debug, Clone)]
pub struct Erc20HarnessParams {
    pub alice_initial_ether: EtherQuantity,
    pub htlc_refund_timestamp: Timestamp,
    pub relative_timelock: Duration,
    pub htlc_secret_hash: SecretHash,
    pub alice_initial_tokens: U256,
    pub htlc_token_value: U256,
}

impl Default for Erc20HarnessParams {
    fn default() -> Self {
        let current_timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("SystemTime::duration_since failed")
            .as_secs() as u32;
        let relative_timelock = 10;
        let htlc_refund_timestamp = Timestamp(current_timestamp + relative_timelock);

        Self {
            alice_initial_ether: EtherQuantity::from_eth(1.0),
            htlc_refund_timestamp,
            relative_timelock: Duration::from_secs(relative_timelock as u64),
            htlc_secret_hash: Secret::from_vec(SECRET).unwrap().hash(),
            alice_initial_tokens: U256::from(1000),
            htlc_token_value: U256::from(400),
        }
    }
}

impl Erc20HarnessParams {
    pub fn with_secret_hash(self, secret_hash: SecretHash) -> Self {
        Self {
            htlc_secret_hash: secret_hash,
            ..self
        }
    }
}

pub fn erc20_harness<D: Docker>(
    docker: &D,
    params: Erc20HarnessParams,
) -> (
    Address,
    Address,
    Address,
    Erc20Htlc,
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

    let token_contract = alice_client.deploy_erc20_token_contract();

    alice_client.mint_tokens(token_contract, params.alice_initial_tokens, alice);

    let erc20_htlc = Erc20Htlc::new(
        params.htlc_refund_timestamp,
        alice,
        bob,
        params.htlc_secret_hash,
        token_contract,
        Erc20Quantity(params.htlc_token_value),
    );

    let tx_id = alice_client.deploy_htlc(erc20_htlc.clone(), U256::from(0));

    (
        alice,
        bob,
        alice_client.get_contract_address(tx_id),
        erc20_htlc,
        token_contract,
        alice_client,
        event_loop,
        container,
    )
}
