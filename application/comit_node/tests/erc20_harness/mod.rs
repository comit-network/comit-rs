use comit_node::{
    gas_price_service::StaticGasPriceService,
    swap_protocols::rfc003::ledger_htlc_service::{
        self, Erc20HtlcParams, EthereumService, LedgerHtlcService,
    },
};
use common_types::{seconds::Seconds, secret::Secret};
use env_logger;
use ethereum_support::{EthereumQuantity, ToEthereumAddress};
use ethereum_wallet::fake::StaticFakeWallet;
use parity_client::ParityClient;
use secp256k1_support::KeyPair;
use std::{sync::Arc, time::Duration};
use tc_parity_parity::ParityEthereum;
use tc_web3_client;
use testcontainers::{clients::DockerCli, Container, Docker};
use web3::{
    transports::EventLoopHandle,
    types::{Address, U256},
};

pub struct Erc20TestHarnessParams {
    pub alice_tokens: U256,
    pub htlc_timeout: Duration,
    pub htlc_value: U256,
    pub htlc_secret: [u8; 32],
}

pub fn harness(
    params: Erc20TestHarnessParams,
) -> (
    Address,
    Address,
    Result<Address, ledger_htlc_service::Error>,
    Address,
    ParityClient,
    EventLoopHandle,
    Container<DockerCli, ParityEthereum>,
) {
    let _ = env_logger::try_init();

    let (alice_keypair, alice) =
        new_account("63be4b0d638d44b5fee5b050ab0beeeae7b68cde3d829a3321f8009cdd76b992");
    let (_, bob) = new_account("f8218ebf6e2626bd1415c18321496e0c5725f0e1d774c7c2eab69f7650ad6e82");

    let container = DockerCli::new().run(ParityEthereum::default());
    let (event_loop, web3) = tc_web3_client::new(&container);

    let client = ParityClient::new(web3);
    client.give_eth_to(alice, EthereumQuantity::from_eth(1.0));

    let token_contract = client.deploy_erc20_token_contract();

    client.mint_tokens(token_contract, params.alice_tokens, alice);

    let ethereum_service = EthereumService::new(
        Arc::new(StaticFakeWallet::from_key_pair(alice_keypair.clone())),
        Arc::new(StaticGasPriceService::default()),
        Arc::new(tc_web3_client::new(&container)),
        0,
    );

    let htlc_params = Erc20HtlcParams {
        refund_address: alice,
        success_address: bob,
        time_lock: Seconds::from(params.htlc_timeout),
        amount: params.htlc_value,
        secret_hash: Secret::from(params.htlc_secret).hash(),
        token_contract_address: token_contract,
    };

    let htlc = ethereum_service
        .deploy_htlc(htlc_params)
        .map(|tx_id| client.get_contract_address(tx_id.clone()));

    (
        alice,
        bob,
        htlc,
        token_contract,
        client,
        event_loop,
        container,
    )
}

fn new_account(secret_key: &str) -> (KeyPair, Address) {
    let keypair = KeyPair::from_secret_key_hex(secret_key).unwrap();
    let address = keypair.public_key().to_ethereum_address();

    (keypair, address)
}
