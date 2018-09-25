extern crate bitcoin_htlc;
extern crate bitcoin_support;
extern crate ethereum_support;
extern crate event_store;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bitcoin_rpc_client;
extern crate comit_node;
extern crate common_types;
extern crate env_logger;
extern crate ethereum_wallet;
extern crate ganache_rust_web3;
extern crate hex;
extern crate reqwest;
#[macro_use]
extern crate serde_json;
extern crate futures;
extern crate ganp;
extern crate gotham;
extern crate hyper;
extern crate tc_trufflesuite_ganachecli;
extern crate tc_web3_client;
extern crate testcontainers;
extern crate uuid;
use bitcoin_rpc_client::TransactionId;
use bitcoin_support::Network;

mod mocks;

use comit_node::{
    bitcoin_fee_service::StaticBitcoinFeeService,
    comit_client::{self, FakeClient, FakeFactory},
    gas_price_service::StaticGasPriceService,
    gotham_factory::create_gotham_router,
    rocket_factory::create_rocket_instance,
    swap_protocols::rfc003::ledger_htlc_service::{BitcoinService, EthereumService},
    swaps::common::TradeId,
};
use common_types::seconds::Seconds;
use ethereum_wallet::fake::StaticFakeWallet;
use event_store::InMemoryEventStore;
use futures::{
    stream::Stream,
    sync::oneshot::{self, Receiver},
    Future,
};
use ganp::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003,
};
use gotham::test::TestServer;
use hex::FromHex;
use hyper::{header::ContentType, mime::APPLICATION_JSON, StatusCode};
use mocks::{BitcoinRpcClientMock, OfferResponseBody, RedeemDetails, StaticEthereumApi};
use std::{net::SocketAddr, str::FromStr, sync::Arc};

fn build_test_server() -> (TestServer, Arc<FakeFactory>) {
    use ganp::{ledger::Ledger, rfc003, swap};
    let event_store = Arc::new(InMemoryEventStore::new());
    let fake_factory = Arc::new(FakeFactory::new());
    let router = create_gotham_router(
        event_store,
        fake_factory.clone(),
        SocketAddr::from_str("127.0.0.1:4242").unwrap(),
    );
    (TestServer::new(router).unwrap(), fake_factory)
}

#[test]
fn get_non_existent_swap() {
    let (test_server, _) = build_test_server();
    let id = TradeId::default();

    let response = test_server
        .client()
        .get(format!("http://localhost/swap/{}", id).as_str())
        .perform()
        .unwrap();

    assert_eq!(response.status(), StatusCode::NotFound)
}

#[test]
fn api_http_api_swap() {
    let _ = env_logger::try_init();
    let (test_server, fake_factory) = build_test_server();

    let response = test_server
        .client()
        .post(
            "http://localhost/swap",
            json!(
            {
                "source_ledger"  : {
                    "value" : "Bitcoin",
                    "identity" : "ac2db2f2615c81b83fe9366450799b4992931575",
                },
                "target_ledger" : {
                    "value" : "Ethereum",
                    "identity" : "0x00a329c0648769a73afac7f9381e08fb43dbea72"
                },
                "source_asset" : {
                    "value" : "Bitcoin",
                    "quantity" : "100000000"
                },
                "target_asset" : {
                    "value" : "Ether",
                    "quantity" : "1000000000000000000"
                }
            }
        ).to_string(),
            APPLICATION_JSON,
        )
        .perform()
        .unwrap();

    assert_eq!(response.status(), StatusCode::Created);
    {
        let headers = response.headers();
        println!("{:?}", headers);
        assert!(headers.has::<ContentType>());
        let content_type = headers.get::<ContentType>().unwrap();
        assert_eq!(content_type, &ContentType::json());
    }

    #[derive(Deserialize, Debug)]
    struct SwapCreated {
        pub id: TradeId,
    }

    let swap_created =
        serde_json::from_slice::<SwapCreated>(response.read_body().unwrap().as_ref());

    assert!(swap_created.is_ok());

    let swap_created = swap_created.unwrap();

    {
        #[derive(Deserialize)]
        struct SwapPending {
            pub status: String,
        }
        let response = test_server
            .client()
            .get(format!("http://localhost/swap/{}", swap_created.id).as_str())
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let get_swap =
            serde_json::from_slice::<SwapPending>(response.read_body().unwrap().as_ref()).unwrap();

        assert_eq!(get_swap.status, "pending");
    }

    //=== SIMULATE THE RESPONSE ===

    fake_factory
        .fake_client
        .resolve_request(rfc003::AcceptResponse::<Bitcoin, Ethereum> {
            target_ledger_refund_identity: ethereum_support::Address::from_str(
                "b3474ca43d419fc54110f7dbc4626f1a2f86b4ab",
            ).unwrap(),
            source_ledger_success_identity: bitcoin_support::PubkeyHash::from_hex(
                "2107b76566056263e6f281f3a991b6651284bc76",
            ).unwrap(),
            target_ledger_lock_duration: Seconds::new(60 * 60 * 24),
        });

    {
        #[derive(Deserialize)]
        struct SwapAccepted {
            pub status: String,
            pub to_fund: bitcoin_support::Address,
        }

        let response = test_server
            .client()
            .get(format!("http://localhost/swap/{}", swap_created.id).as_str())
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let get_swap =
            serde_json::from_slice::<SwapAccepted>(response.read_body().unwrap().as_ref()).unwrap();

        assert_eq!(get_swap.status, "accepted");
    }
}

// sha256 of htlc script: e6877a670b46b9913bdaed47084f2db8983c2a22c473f0aea1fa5c2ebc4fd8d4

// fn create_rocket_client() -> Client {
//     let bitcoin_fee_service = Arc::new(StaticBitcoinFeeService::new(50.0));
//     let bob_success_address =
//         bitcoin_support::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap();
//     let bitcoin_service = Arc::new(BitcoinService::new(
//         Arc::new(BitcoinRpcClientMock::new(
//             TransactionId::from_str(
//                 "d54994ece1d11b19785c7248868696250ab195605b469632b7bd68130e880c9a",
//             ).unwrap(),
//         )),
//         bitcoin_support::Network::Regtest,
//         bitcoin_fee_service.clone(),
//         bob_success_address,
//     ));

//     let rocket = create_rocket_instance(
//         Arc::new(InMemoryEventStore::new()),
//         Arc::new(EthereumService::new(
//             Arc::new(StaticFakeWallet::account0()),
//             Arc::new(StaticGasPriceService::default()),
//             Arc::new(StaticEthereumApi),
//             0,
//         )),
//         bitcoin_service,
//         "e7b6bfabddfaeb2c016b334a5322e4327dc5e499".into(),
//         bitcoin_support::PrivateKey::from_str(
//             "cR6U4gNiCQsPo5gLNP2w6QsLTZkvCGEijhYVPZVhnePQKjMwmas8",
//         ).unwrap()
//             .secret_key()
//             .clone()
//             .into(),
//         Network::Regtest,
//         Arc::new(comit_client::FakeFactory {}),
//         SocketAddr::from_str("127.0.0.1:4242").unwrap(),
//     );
//     rocket::local::Client::new(rocket).unwrap()
// }

// Secret: 12345678901234567890123456789012
// Secret hash: 51a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c

// Sender address: bcrt1qryj6ya9vqpph8w65992nhk64cs890vfy0khsfg
// Sender pubkey: 020c04eb8cb87485501e30b656f37439ea7866d7c58b3c38161e5793b68e712356
// Sender pubkey hash: 1925a274ac004373bb5429553bdb55c40e57b124

// Recipient address: bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap
// Recipient pubkey: 0298e113cc06bc862ac205f2c0f27ee8c0de98d0716537bbf74e2ea6f38a84d5dc
// Recipient pubkey hash: c021f17be99c6adfbcba5d38ee0d292c0399d2f5

// htlc script: 63a82051a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c8876a914c021f17be99c6adfbcba5d38ee0d292c0399d2f567028403b17576a9141925a274ac004373bb5429553bdb55c40e57b1246888ac
