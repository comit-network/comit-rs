#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc;
extern crate bitcoin_support;
extern crate common_types;
extern crate env_logger;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate event_store;
extern crate exchange_service;
extern crate hex;
extern crate rocket;
extern crate rocket_contrib;
extern crate secp256k1_support;
extern crate serde;
extern crate serde_json;

use bitcoin_rpc::BlockHeight;
use bitcoin_support::Network;
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    secret::Secret,
    TradingSymbol,
};
use ethereum_support::{web3, Bytes, H256};
use ethereum_wallet::fake::StaticFakeWallet;
use event_store::{EventStore, InMemoryEventStore};
use exchange_service::{
    bitcoin_fee_service::StaticBitcoinFeeService,
    ethereum_service::{self, BlockingEthereumApi},
    gas_price_service::StaticGasPriceService,
    rocket_factory::create_rocket_instance,
    swaps::{
        common::TradeId,
        events::{OfferCreated, OrderTaken, TradeFunded},
    },
    treasury_api_client::FakeApiClient,
};
use hex::FromHex;
use rocket::{
    http::{ContentType, Status},
    local::{Client, LocalResponse},
};
use secp256k1_support::KeyPair;
use serde::Deserialize;
use std::{str::FromStr, sync::Arc, time::Duration};

fn request_offer(client: &mut Client) -> LocalResponse {
    let request = client
        .post("/trades/ETH-BTC/buy-offers")
        .header(ContentType::JSON)
        .body(r#"{ "amount": 42 }"#);
    request.dispatch()
}

fn request_order<'a>(client: &'a mut Client, uid: &TradeId) -> LocalResponse<'a> {
    let request = client
        .post(format!("/trades/ETH-BTC/{}/sell-orders", uid).to_string())
        .header(ContentType::JSON)
        .body(
            r#"{
                    "contract_secret_lock": "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec",
                    "client_refund_address": "0x956abb53d3ccbf24cf2f8c6e334a56d4b6c50440",
                    "client_success_address": "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
                    "client_contract_time_lock": 24
                  }"#,
        );
    request.dispatch()
}

trait DeserializeAsJson {
    fn body_json<T>(&mut self) -> T
    where
        for<'de> T: Deserialize<'de>;
}

impl<'r> DeserializeAsJson for LocalResponse<'r> {
    fn body_json<T>(&mut self) -> T
    where
        for<'de> T: Deserialize<'de>,
    {
        let body = self.body().unwrap().into_inner();

        serde_json::from_reader(body).unwrap()
    }
}

struct StaticEthereumApi;

impl BlockingEthereumApi for StaticEthereumApi {
    fn send_raw_transaction(&self, _rlp: Bytes) -> Result<H256, web3::Error> {
        Ok(H256::new())
    }
}

fn create_rocket_client(event_store: InMemoryEventStore<TradeId>) -> Client {
    let rocket = create_rocket_instance(
        Arc::new(FakeApiClient),
        event_store,
        Arc::new(ethereum_service::EthereumService::new(
            Arc::new(StaticFakeWallet::account0()),
            Arc::new(StaticGasPriceService::default()),
            Arc::new(StaticEthereumApi),
            0,
        )),
        Arc::new(bitcoin_rpc::BitcoinStubClient::new()),
        "e7b6bfabddfaeb2c016b334a5322e4327dc5e499".into(),
        bitcoin_support::PrivateKey::from_str(
            "cR6U4gNiCQsPo5gLNP2w6QsLTZkvCGEijhYVPZVhnePQKjMwmas8",
        ).unwrap()
            .secret_key()
            .clone()
            .into(),
        bitcoin_support::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap(),
        Network::BitcoinCoreRegtest,
        Arc::new(StaticBitcoinFeeService::new(50.0)),
    );
    rocket::local::Client::new(rocket).unwrap()
}

//series of events is as follows:
// OfferCreated buy ETH for BTC -> OrderTaken ETH for BTC-> TradeFunded BTC from trader -> ContractDeployed ETH from exchange

#[test]
fn given_an_accepted_trade_when_provided_with_funding_tx_should_deploy_htlc() {
    let _ = env_logger::try_init();
    let event_store = InMemoryEventStore::new();

    let trade_id = TradeId::new();

    let offer_created: OfferCreated<Bitcoin, Ethereum> = OfferCreated::new(
        0.1,
        bitcoin_support::BitcoinQuantity::from_bitcoin(1.0),
        ethereum_support::EthereumQuantity::from_eth(10.0),
        TradingSymbol::ETH_BTC,
    );

    event_store.add_event(trade_id.clone(), offer_created);
    let bytes = b"hello world, you are beautiful!!";
    let secret = Secret::from(*bytes);

    let secret_key_data = <[u8; 32]>::from_hex(
        "e8aafba2be13ee611059bc756878933bee789cc1aec7c35e23054a44d071c80b",
    ).unwrap();
    let keypair = KeyPair::from_secret_key_slice(&secret_key_data).unwrap();

    let order_taken: OrderTaken<Bitcoin, Ethereum> = OrderTaken {
        uid: trade_id,
        contract_secret_lock: secret.hash(),
        client_contract_time_lock: Duration::new(60 * 60 * 12, 0),
        exchange_contract_time_lock: BlockHeight::new(24u32),
        client_refund_address: ethereum_support::Address::from_str(
            "1111111111111111111111111111111111111111",
        ).unwrap(),
        client_success_address: bitcoin_support::Address::from_str(
            "2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL",
        ).unwrap(),
        exchange_refund_address: bitcoin_support::Address::from_str(
            "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
        ).unwrap(),
        exchange_success_address: ethereum_support::Address::from_str(
            "2222222222222222222222222222222222222222",
        ).unwrap(),
        exchange_success_keypair: keypair,
    };

    event_store.add_event(trade_id, order_taken);

    //    let trade_funded: TradeFunded<Ethereum> = TradeFunded {
    //        uid: trade_id,
    //        htlc_identifier: ethereum_support::Address::from_str("2222222222222222222222222222222222222222").unwrap(),
    //    };
    //    event_store.add_event(trade_id, trade_funded);
    //
    let mut client = create_rocket_client(event_store);

    let response = {
        let request = client
            .post(format!("/trades/ETH-BTC/{}/sell-order-htlc-funded", trade_id).to_string())
            .header(ContentType::JSON)
            .body(r#" "0x3333333333333333333333333333333333333333" "#);
        request.dispatch()
    };

    println!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
}
