extern crate http;
extern crate ledger_query_service;
extern crate pretty_env_logger;
extern crate rocket;
extern crate serde_json;
extern crate spectral;
#[macro_use]
extern crate serde_derive;
extern crate ethereum_support;
extern crate secp256k1_support;
extern crate tc_web3_client;
extern crate testcontainers;

use ethereum_support::{
    web3::types::{Address, TransactionRequest, U256},
    Future, ToEthereumAddress,
};
use http::Uri;
use ledger_query_service::{
    DefaultBlockProcessor, EthereumWeb3BlockPoller, InMemoryQueryRepository,
    InMemoryQueryResultRepository, LinkFactory,
};
use rocket::{
    http::{ContentType, Status},
    local::Client,
};
use secp256k1_support::KeyPair;
use spectral::prelude::*;
use std::{sync::Arc, time::Duration};
use testcontainers::{clients::Cli, images::parity_parity::ParityEthereum, Docker};

#[derive(Deserialize, Debug)]
struct QueryResponse {
    matching_transactions: Vec<String>,
}

fn new_account(secret_key: &str) -> (KeyPair, Address) {
    let keypair = KeyPair::from_secret_key_hex(secret_key).unwrap();
    let address = keypair.public_key().to_ethereum_address();

    (keypair, address)
}

#[test]
fn given_to_address_query_when_matching_transaction_is_processed_returns_result() {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();

    let container = docker.run(ParityEthereum::default());
    let (_event_loop, web3) = tc_web3_client::new(&container);

    let link_factory = LinkFactory::new("http", "localhost", Some(8000));
    let transaction_query_repository = Arc::new(InMemoryQueryRepository::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let block_query_repository = Arc::new(InMemoryQueryRepository::default());
    let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let transaction_processor = DefaultBlockProcessor::new(
        transaction_query_repository.clone(),
        block_query_repository.clone(),
        transaction_query_result_repository.clone(),
        block_query_result_repository.clone(),
    );

    let server = ledger_query_service::server_builder::ServerBuilder::create(
        rocket::Config::development().unwrap(),
        link_factory,
    ).register_ethereum(
        transaction_query_repository,
        transaction_query_result_repository,
        block_query_repository,
        block_query_result_repository,
    ).build();
    let client = Client::new(server).unwrap();

    let (_alice_keypair, alice) =
        new_account("63be4b0d638d44b5fee5b050ab0beeeae7b68cde3d829a3321f8009cdd76b992");
    let (_bob_keypair, bob) =
        new_account("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

    let response = client
        .post("/queries/ethereum/transactions")
        .header(ContentType::JSON)
        .body(r#"{ "to_address" : "0x88f9b82462f6c4bf4a0fb15e5c3971559a316e7f" }"#) // Bob's address
        .dispatch();

    let location_header_value = response.headers().get_one("Location");
    let uri: Uri = location_header_value.unwrap().parse().unwrap();

    let web3_endpoint = format!(
        "http://localhost:{}",
        container.get_host_port(8545).unwrap()
    );

    ::std::thread::spawn(move || {
        let ethereum_poller = EthereumWeb3BlockPoller::new(
            web3_endpoint.as_str(),
            Duration::from_secs(1),
            transaction_processor,
        );

        match ethereum_poller {
            Ok(listener) => listener.start(),
            Err(e) => println!("Failed to start EthereumWeb3BlockPoller! {:?}", e),
        }
    });

    let parity_dev_account: Address = "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap();

    let dev_to_alice_hash = web3
        .personal()
        .send_transaction(
            TransactionRequest {
                from: parity_dev_account.clone(),
                to: Some(alice),
                gas: Some(U256::from(4_000_000u64)),
                gas_price: None,
                value: Some(U256::from(1_000_000u64)),
                data: None,
                nonce: None,
                condition: None,
            },
            "",
        ).wait()
        .unwrap();

    let dev_to_bob_hash = web3
        .personal()
        .send_transaction(
            TransactionRequest {
                from: parity_dev_account.clone(),
                to: Some(bob),
                gas: Some(U256::from(4_000_000u64)),
                gas_price: None,
                value: Some(U256::from(200_000u64)),
                data: None,
                nonce: None,
                condition: None,
            },
            "",
        ).wait()
        .unwrap();

    // Wait for the polling to happen on the other thread
    ::std::thread::sleep(Duration::from_secs(2));

    let mut get_response = client.get(uri.path()).dispatch();
    assert_that(&get_response.status()).is_equal_to(Status::Ok);

    let body = get_response.body_bytes();
    let body = assert_that(&body).is_some().subject;
    let body = serde_json::from_slice::<QueryResponse>(body);
    let body = assert_that(&body).is_ok().subject;

    assert_that(body)
        .map(|b| &b.matching_transactions)
        .does_not_contain(format!("{:?}", dev_to_alice_hash));

    assert_that(body)
        .map(|b| &b.matching_transactions)
        .contains(format!("{:?}", dev_to_bob_hash));
}
