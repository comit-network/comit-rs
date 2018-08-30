#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate common_types;
extern crate env_logger;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate event_store;
extern crate exchange_service;
extern crate hex;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate secp256k1_support;
extern crate serde;
extern crate serde_json;

mod mocks;

use bitcoin_rpc_client::{BlockHeight, TransactionId};
use bitcoin_support::Network;
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    seconds::Seconds,
    secret::Secret,
    TradingSymbol,
};
use ethereum_support::{web3, Bytes, H256};
use ethereum_wallet::fake::StaticFakeWallet;
use event_store::{EventStore, InMemoryEventStore};
use exchange_service::{
    bitcoin_fee_service::StaticBitcoinFeeService,
    bitcoin_service::BitcoinService,
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
use mocks::BitcoinRpcClientMock;
use rocket::{
    http::{ContentType, Status},
    local::{Client, LocalResponse},
};
use secp256k1_support::KeyPair;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

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

fn create_rocket_client(
    event_store: InMemoryEventStore<TradeId>,
    bitcoin_service: BitcoinService,
) -> Client {
    let rocket = create_rocket_instance(
        Arc::new(FakeApiClient),
        event_store,
        Arc::new(ethereum_service::EthereumService::new(
            Arc::new(StaticFakeWallet::account0()),
            Arc::new(StaticGasPriceService::default()),
            Arc::new(StaticEthereumApi),
            0,
        )),
        Arc::new(bitcoin_service),
        Arc::new(bitcoin_rpc_client::BitcoinStubClient::new()),
        "e7b6bfabddfaeb2c016b334a5322e4327dc5e499".into(),
        bitcoin_support::PrivateKey::from_str(
            "cR6U4gNiCQsPo5gLNP2w6QsLTZkvCGEijhYVPZVhnePQKjMwmas8",
        ).unwrap()
            .secret_key()
            .clone()
            .into(),
        bitcoin_support::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap(),
        Network::Regtest,
        Arc::new(StaticBitcoinFeeService::new(50.0)),
    );
    rocket::local::Client::new(rocket).unwrap()
}

fn mock_offer_created(event_store: &InMemoryEventStore<TradeId>, trade_id: TradeId) {
    let offer_created: OfferCreated<Bitcoin, Ethereum> = OfferCreated::new(
        0.1,
        bitcoin_support::BitcoinQuantity::from_bitcoin(1.0),
        ethereum_support::EthereumQuantity::from_eth(10.0),
        TradingSymbol::ETH_BTC,
    );
    event_store
        .add_event(trade_id.clone(), offer_created)
        .unwrap();
}

fn mock_order_taken(event_store: &InMemoryEventStore<TradeId>, trade_id: TradeId) {
    let bytes = b"hello world, you are beautiful!!";
    let secret = Secret::from(*bytes);

    let secret_key_data = <[u8; 32]>::from_hex(
        "e8aafba2be13ee611059bc756878933bee789cc1aec7c35e23054a44d071c80b",
    ).unwrap();
    let keypair = KeyPair::from_secret_key_slice(&secret_key_data).unwrap();

    let order_taken: OrderTaken<Bitcoin, Ethereum> = OrderTaken {
        uid: trade_id,
        contract_secret_lock: secret.hash(),
        client_contract_time_lock: Seconds::new(60 * 60 * 12),
        exchange_contract_time_lock: BlockHeight::new(24u32),
        client_refund_address: ethereum_support::Address::from_str(
            "2d72ccd2f36173d945bc7247b29b60e5d5d0ca5e", // privkey: 5fce23dbb7656edea89728e2f5a95ea288b9c0d570a2fb839f0c11be6b55c0ab
        ).unwrap(),
        client_success_address: bitcoin_support::Address::from_str(
            "bc1q5p6eyvxld0p2c93fwccw436z9f830v0krsf9ux", //privkey: b2253c744dffb1c6df0465716059d13076780ef184afe1199d7f4a3cb627c7b2
        ).unwrap(),
        exchange_refund_address: bitcoin_support::Address::from_str(
            "bc1q92ec9ycs65fd3xcxxh5wvwzz5cz6jvpthjdxx6", //privkey: e5a2d87ea2c6af42dbc95fbb08d345a4f5bf8dfbf25dc67834a1f5af01729eab
        ).unwrap(),
        exchange_success_address: ethereum_support::Address::from_str(
            "77b0f5692ae5662cdd3f3187774367ad47c53b61", // privkey: 0829b16159b596db867bd9f696e7c0b7c32b0fee7f6379ce15f14f4b355ee0ce
        ).unwrap(),
        exchange_success_keypair: keypair,
    };
    event_store.add_event(trade_id, order_taken).unwrap();
}

fn mock_trade_funded(event_store: &InMemoryEventStore<TradeId>, trade_id: TradeId) {
    let trade_funded: TradeFunded<Bitcoin, Ethereum> = TradeFunded::new(
        trade_id,
        ethereum_support::Address::from_str("2222222222222222222222222222222222222222").unwrap(),
    );
    event_store.add_event(trade_id, trade_funded).unwrap();
}

#[test]
fn given_an_accepted_trade_when_provided_with_funding_tx_should_deploy_htlc() {
    let _ = env_logger::try_init();
    let bitcoin_fee_service = Arc::new(StaticBitcoinFeeService::new(50.0));
    let exchange_success_address =
        bitcoin_support::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap();

    let bitcoin_service = BitcoinService::new(
        Arc::new(BitcoinRpcClientMock::new(
            TransactionId::from_str(
                "d54994ece1d11b19785c7248868696250ab195605b469632b7bd68130e880c9a",
            ).unwrap(),
        )),
        bitcoin_support::Network::Regtest,
        bitcoin_fee_service.clone(),
        exchange_success_address,
    );
    let event_store = InMemoryEventStore::new();

    let trade_id = TradeId::new();

    mock_offer_created(&event_store, trade_id);
    mock_order_taken(&event_store, trade_id);
    let client = create_rocket_client(event_store, bitcoin_service);

    let response = {
        let request = client
            .post(format!("/trades/ETH-BTC/{}/sell-order-htlc-funded", trade_id).to_string())
            .header(ContentType::JSON)
            .body(r#" "0x3333333333333333333333333333333333333333" "#);
        request.dispatch()
    };

    assert_eq!(response.status(), Status::Ok);
}

#[derive(Serialize)]
pub struct RedeemETHNotificationBody {
    pub secret: Secret,
}

#[test]
fn given_an_deployed_htlc_and_secret_should_redeem_htlc() {
    let _ = env_logger::try_init();
    let bitcoin_fee_service = Arc::new(StaticBitcoinFeeService::new(50.0));
    let exchange_success_address =
        bitcoin_support::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap();

    let bitcoin_service = BitcoinService::new(
        Arc::new(BitcoinRpcClientMock::new(
            TransactionId::from_str(
                "d54994ece1d11b19785c7248868696250ab195605b469632b7bd68130e880c9a",
            ).unwrap(),
        )),
        bitcoin_support::Network::Regtest,
        bitcoin_fee_service.clone(),
        exchange_success_address,
    );
    let event_store = InMemoryEventStore::new();

    let trade_id = TradeId::new();

    mock_offer_created(&event_store, trade_id);
    mock_order_taken(&event_store, trade_id);
    mock_trade_funded(&event_store, trade_id);

    let client = create_rocket_client(event_store, bitcoin_service);

    let bytes = b"hello world, you are beautiful!!";
    let secret = Secret::from(*bytes);
    let redeem_body = RedeemETHNotificationBody { secret };
    let response = {
        let request = client
            .post(format!("/trades/ETH-BTC/{}/sell-order-secret-revealed", trade_id).to_string())
            .header(ContentType::JSON)
            .body(serde_json::to_string(&redeem_body).unwrap());
        request.dispatch()
    };
    assert_eq!(response.status(), Status::Ok);
}
