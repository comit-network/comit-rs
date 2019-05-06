use crate::{
    ethereum_wallet::InMemoryWallet,
    htlc_harness::{new_account, SECRET_HASH},
    parity_client::ParityClient,
};
use blockchain_contracts::{
    ethereum::rfc003::{erc20::SECRET_HASH_LENGTH, Erc20Htlc},
    rfc003::timestamp::Timestamp,
};
use ethereum_support::{
    web3::{
        transports::EventLoopHandle,
        types::{Address, U256},
    },
    EtherQuantity,
};
use std::sync::Arc;
use tc_web3_client;
use testcontainers::{images::parity_parity::ParityEthereum, Container, Docker};

#[derive(Debug, Clone)]
pub struct Erc20HarnessParams {
    pub alice_initial_ether: EtherQuantity,
    pub htlc_refund_timestamp: Timestamp,
    pub htlc_secret_hash: [u8; SECRET_HASH_LENGTH],
    pub alice_initial_tokens: U256,
    pub htlc_token_value: U256,
}

impl Default for Erc20HarnessParams {
    fn default() -> Self {
        let mut secret_hash = [0; SECRET_HASH_LENGTH];
        secret_hash.copy_from_slice(&hex::decode(SECRET_HASH).unwrap()[..SECRET_HASH_LENGTH]);

        Self {
            alice_initial_ether: EtherQuantity::from_eth(1.0),
            htlc_refund_timestamp: Timestamp::now().plus(10),
            htlc_secret_hash: secret_hash,
            alice_initial_tokens: U256::from(1000),
            htlc_token_value: U256::from(400),
        }
    }
}

impl Erc20HarnessParams {
    pub fn with_secret_hash(self, secret_hash: [u8; SECRET_HASH_LENGTH]) -> Self {
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
        params.htlc_token_value,
    )
    .expect("Compile the ERC20 HTLC");

    let tx_id = alice_client.deploy_htlc(erc20_htlc.clone().into(), U256::from(0));

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
