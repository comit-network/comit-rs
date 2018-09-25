#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate comit_node;
extern crate common_types;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate event_store;
extern crate pretty_env_logger;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate secp256k1_support;
extern crate serde;
extern crate serde_json;
extern crate uuid;
#[macro_use]
extern crate serde_derive;
extern crate comit_wallet;
extern crate hex;
extern crate tc_web3_client;
extern crate testcontainers;

mod mocks;

use bitcoin_rpc_client::TransactionId;
use bitcoin_support::{Blocks, Network};
use comit_node::{
    bitcoin_fee_service::StaticBitcoinFeeService,
    comit_node_api_client::FakeApiClient as FakeComitNodeApiClient,
    gas_price_service::StaticGasPriceService,
    rocket_factory::create_rocket_instance,
    swap_protocols::{
        ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
        rfc003::ledger_htlc_service::{BitcoinService, BlockingEthereumApi, EthereumService},
    },
    swaps::{
        bob_events::{OrderTaken, TradeFunded},
        common::TradeId,
    },
};
use comit_wallet::fake_key_store::FakeKeyStoreFactory;
use common_types::{seconds::Seconds, secret::Secret};
use ethereum_support::{web3, Bytes, H256};
use ethereum_wallet::fake::StaticFakeWallet;
use event_store::{EventStore, InMemoryEventStore};
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
    let bob_key_store = Arc::new(FakeKeyStoreFactory::create());

    let api_client = FakeComitNodeApiClient::new();

    let rocket = create_rocket_instance(
        Arc::new(event_store),
        Arc::new(EthereumService::new(
            Arc::new(StaticFakeWallet::account0()),
            Arc::new(StaticGasPriceService::default()),
            Arc::new(StaticEthereumApi),
            0,
        )),
        Arc::new(bitcoin_service),
        bob_key_store,
        Network::Regtest,
        Arc::new(api_client),
        "0.0.0.0".into(),
        8080,
        true,
    );
    rocket::local::Client::new(rocket).unwrap()
}

fn mock_order_taken(event_store: &InMemoryEventStore<TradeId>, trade_id: TradeId) {
    let bytes = b"hello world, you are beautiful!!";
    let secret = Secret::from(*bytes);

    let secret_key_data =
        <[u8; 32]>::from_hex("e8aafba2be13ee611059bc756878933bee789cc1aec7c35e23054a44d071c80b")
            .unwrap();
    let keypair = KeyPair::from_secret_key_slice(&secret_key_data).unwrap();

    let order_taken: OrderTaken<Bitcoin, Ethereum> = OrderTaken {
        uid: trade_id,
        contract_secret_lock: secret.hash(),
        alice_contract_time_lock: Seconds::new(60 * 60 * 12),
        bob_contract_time_lock: Blocks::from(24u32),
        alice_refund_address: ethereum_support::Address::from_str(
            "2d72ccd2f36173d945bc7247b29b60e5d5d0ca5e", // privkey: 5fce23dbb7656edea89728e2f5a95ea288b9c0d570a2fb839f0c11be6b55c0ab
        ).unwrap(),
        alice_success_address: bitcoin_support::Address::from_str(
            "bc1q5p6eyvxld0p2c93fwccw436z9f830v0krsf9ux", //privkey: b2253c744dffb1c6df0465716059d13076780ef184afe1199d7f4a3cb627c7b2
        ).unwrap(),
        bob_refund_address: bitcoin_support::Address::from_str(
            "bc1q92ec9ycs65fd3xcxxh5wvwzz5cz6jvpthjdxx6", //privkey: e5a2d87ea2c6af42dbc95fbb08d345a4f5bf8dfbf25dc67834a1f5af01729eab
        ).unwrap(),
        bob_success_address: ethereum_support::Address::from_str(
            "77b0f5692ae5662cdd3f3187774367ad47c53b61", // privkey: 0829b16159b596db867bd9f696e7c0b7c32b0fee7f6379ce15f14f4b355ee0ce
        ).unwrap(),
        bob_success_keypair: keypair,
        buy_amount: bitcoin_support::BitcoinQuantity::from_satoshi(10000000),
        sell_amount: ethereum_support::EthereumQuantity::from_eth(0.000000000001),
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

fn create_bitcoin_service() -> BitcoinService {
    let bitcoin_fee_service = Arc::new(StaticBitcoinFeeService::new(50.0));
    let bob_success_address =
        bitcoin_support::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap();
    BitcoinService::new(
        Arc::new(BitcoinRpcClientMock::new(
            TransactionId::from_str(
                "d54994ece1d11b19785c7248868696250ab195605b469632b7bd68130e880c9a",
            ).unwrap(),
        )),
        bitcoin_support::Network::Regtest,
        bitcoin_fee_service.clone(),
        bob_success_address,
    )
}

#[test]
fn given_an_accepted_trade_when_provided_with_funding_tx_should_deploy_htlc() {
    let _ = pretty_env_logger::try_init();
    let bitcoin_service = create_bitcoin_service();

    let event_store = InMemoryEventStore::new();
    let trade_id = Default::default();

    mock_order_taken(&event_store, trade_id);
    let client = create_rocket_client(event_store, bitcoin_service);

    let response = {
        let request = client
            .post(format!("/ledger/trades/ETH-BTC/{}/sell-order-htlc-funded", trade_id).to_string())
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
    let _ = pretty_env_logger::try_init();
    let bitcoin_service = create_bitcoin_service();

    let event_store = InMemoryEventStore::new();

    let trade_id = Default::default();

    mock_order_taken(&event_store, trade_id);
    mock_trade_funded(&event_store, trade_id);

    let client = create_rocket_client(event_store, bitcoin_service);

    let bytes = b"hello world, you are beautiful!!";
    let secret = Secret::from(*bytes);
    let redeem_body = RedeemETHNotificationBody { secret };
    let response = {
        let request = client
            .post(format!(
                "/ledger/trades/ETH-BTC/{}/sell-order-secret-revealed",
                trade_id
            )).header(ContentType::JSON)
            .body(serde_json::to_string(&redeem_body).unwrap());
        request.dispatch()
    };
    assert_eq!(response.status(), Status::Ok);
}
