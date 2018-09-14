extern crate http;
extern crate ledger_query_service;
extern crate pretty_env_logger;
extern crate rocket;
extern crate serde_json;
extern crate spectral;
#[macro_use]
extern crate serde_derive;
extern crate bitcoin_support;

use bitcoin_support::{Address, Transaction, TxOut};
use http::Uri;
use ledger_query_service::{
    DefaultTransactionProcessor, InMemoryQueryRepository, InMemoryQueryResultRepository,
    LinkFactory, TransactionProcessor,
};
use rocket::{
    http::{ContentType, Status},
    local::Client,
};
use spectral::prelude::*;
use std::sync::Arc;

#[test]
fn can_access_query_resource_after_creation() {
    let _ = pretty_env_logger::try_init();

    let link_factory = LinkFactory::new("http", "localhost", Some(8000));
    let query_repository = Arc::new(InMemoryQueryRepository::default());
    let query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    let server = ledger_query_service::server_builder::ServerBuilder::create(
        rocket::Config::development().unwrap(),
        link_factory,
    ).register_bitcoin(query_repository, query_result_repository)
    .build();
    let client = Client::new(server).unwrap();

    let response = client
        .post("/queries/bitcoin")
        .header(ContentType::JSON)
        .body(include_str!("bitcoin_query.json"))
        .dispatch();
    let status = response.status();
    let location_header_value = response.headers().get_one("Location");

    assert_that(&status).is_equal_to(Status::Created);
    assert_that(&location_header_value)
        .is_some()
        .is_equal_to("http://localhost:8000/queries/bitcoin/1");

    let uri: Uri = location_header_value.unwrap().parse().unwrap();

    // Unfortunately, rocket cannot access the resource if we pass the full URL. We therefore have to extract the path in order to pass the test ...
    let response = client.get(uri.path()).dispatch();

    assert_that(&response.status()).is_equal_to(Status::Ok);
}

#[test]
fn given_created_query_when_deleted_is_no_longer_available() {
    let _ = pretty_env_logger::try_init();

    let link_factory = LinkFactory::new("http", "localhost", Some(8000));
    let query_repository = Arc::new(InMemoryQueryRepository::default());
    let query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    let server = ledger_query_service::server_builder::ServerBuilder::create(
        rocket::Config::development().unwrap(),
        link_factory,
    ).register_bitcoin(query_repository, query_result_repository)
    .build();
    let client = Client::new(server).unwrap();

    let response = client
        .post("/queries/bitcoin")
        .header(ContentType::JSON)
        .body(include_str!("bitcoin_query.json"))
        .dispatch();

    let location_header_value = response.headers().get_one("Location");
    let uri: Uri = location_header_value.unwrap().parse().unwrap();

    let delete_response = client.delete(uri.path()).dispatch();
    assert_that(&delete_response.status()).is_equal_to(Status::NoContent);

    let get_after_delete_response = client.get(uri.path()).dispatch();
    assert_that(&get_after_delete_response.status()).is_equal_to(Status::NotFound);
}
#[derive(Deserialize, Debug)]
struct QueryResponse {
    matching_transactions: Vec<String>,
}

#[test]
fn given_query_when_matching_transaction_is_processed_returns_result() {
    let _ = pretty_env_logger::try_init();

    let link_factory = LinkFactory::new("http", "localhost", Some(8000));
    let query_repository = Arc::new(InMemoryQueryRepository::default());
    let query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let transaction_processor =
        DefaultTransactionProcessor::new(query_repository.clone(), query_result_repository.clone());

    let server = ledger_query_service::server_builder::ServerBuilder::create(
        rocket::Config::development().unwrap(),
        link_factory,
    ).register_bitcoin(query_repository, query_result_repository)
    .build();
    let client = Client::new(server).unwrap();

    let response = client
        .post("/queries/bitcoin")
        .header(ContentType::JSON)
        .body(include_str!("bitcoin_query.json"))
        .dispatch();

    let location_header_value = response.headers().get_one("Location");
    let uri: Uri = location_header_value.unwrap().parse().unwrap();

    let address: Address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".parse().unwrap();
    let incoming_transaction = Transaction {
        version: 1,
        lock_time: 0,
        input: Vec::new(),
        output: vec![TxOut {
            value: 0,
            script_pubkey: address.as_ref().script_pubkey(),
        }],
    };

    let tx_id = incoming_transaction.txid();

    transaction_processor.process(&incoming_transaction);

    let mut get_response = client.get(uri.path()).dispatch();
    assert_that(&get_response.status()).is_equal_to(Status::Ok);

    let body = get_response.body_bytes();
    let body = assert_that(&body).is_some().subject;
    let body = serde_json::from_slice::<QueryResponse>(body);
    let body = assert_that(&body).is_ok().subject;

    assert_that(body)
        .map(|b| &b.matching_transactions)
        .contains(tx_id.to_string());
}

#[test]
fn should_reject_malformed_address() {
    let _ = pretty_env_logger::try_init();

    let link_factory = LinkFactory::new("http", "localhost", Some(8000));
    let query_repository = Arc::new(InMemoryQueryRepository::default());
    let query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    let server = ledger_query_service::server_builder::ServerBuilder::create(
        rocket::Config::development().unwrap(),
        link_factory,
    ).register_bitcoin(query_repository, query_result_repository)
    .build();
    let client = Client::new(server).unwrap();

    let response = client
        .post("/queries/bitcoin")
        .header(ContentType::JSON)
        .body(include_str!("bitcoin_query_malformed_to_address.json"))
        .dispatch();

    assert_that(&response.status()).is_equal_to(Status::BadRequest);
}
