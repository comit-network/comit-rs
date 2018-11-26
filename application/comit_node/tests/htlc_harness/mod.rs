use comit_node::swap_protocols::rfc003::{ethereum::Seconds, Secret};
use ethereum_support::{
    web3::{
        transports::EventLoopHandle,
        types::{Address, U256},
    },
    EtherQuantity, ToEthereumAddress,
};
use ethereum_wallet::InMemoryWallet;
use parity_client::{Erc20HtlcDeployParams, EtherHtlcFundingParams, ParityClient};
use pretty_env_logger;
use secp256k1_support::KeyPair;
use std::{sync::Arc, time::Duration};
use tc_web3_client;
use testcontainers::{images::parity_parity::ParityEthereum, Container, Docker};

pub enum HtlcType {
    Erc20 {
        alice_initial_tokens: U256,
        htlc_token_value: U256,
    },
    Eth {
        htlc_eth_value: EtherQuantity,
    },
}

pub struct TestHarnessParams {
    pub alice_initial_ether: EtherQuantity,
    pub htlc_timeout: Duration,
    pub htlc_secret: [u8; 32],
    pub htlc_type: HtlcType,
}

pub fn harness<D: Docker>(
    docker: &D,
    params: TestHarnessParams,
) -> (
    Address,
    Address,
    Address,
    Option<Address>,
    ParityClient,
    EventLoopHandle,
    Container<D, ParityEthereum>,
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

    let (token_contract, htlc) = match params.htlc_type {
        HtlcType::Erc20 {
            alice_initial_tokens,
            htlc_token_value,
        } => {
            let token_contract = alice_client.deploy_erc20_token_contract();

            alice_client.mint_tokens(token_contract, alice_initial_tokens, alice);

            let htlc_params = Erc20HtlcDeployParams {
                refund_address: alice,
                success_address: bob,
                time_lock: Seconds::from(params.htlc_timeout),
                amount: htlc_token_value,
                secret_hash: Secret::from(params.htlc_secret).hash(),
                token_contract_address: token_contract,
            };
            let tx_id = alice_client.deploy_erc20_htlc(htlc_params);

            (
                Some(token_contract),
                alice_client.get_contract_address(tx_id),
            )
        }
        HtlcType::Eth { htlc_eth_value } => {
            let htlc_params = EtherHtlcFundingParams {
                refund_address: alice,
                success_address: bob,
                time_lock: Seconds::from(params.htlc_timeout),
                amount: htlc_eth_value,
                secret_hash: Secret::from(params.htlc_secret).hash(),
            };
            let tx_id = alice_client.deploy_ether_htlc(htlc_params);

            //no funding needed, deployment of the HTLC can also fund it directly
            (None, alice_client.get_contract_address(tx_id))
        }
    };

    (
        alice,
        bob,
        htlc,
        token_contract,
        alice_client,
        event_loop,
        container,
    )
}

fn new_account(secret_key: &str) -> (KeyPair, Address) {
    let keypair = KeyPair::from_secret_key_hex(secret_key).unwrap();
    let address = keypair.public_key().to_ethereum_address();

    (keypair, address)
}
