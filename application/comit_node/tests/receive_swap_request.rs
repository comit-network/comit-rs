extern crate tokio;
extern crate transport_protocol;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate serde_json;
extern crate bitcoin_support;
extern crate comit_node;
extern crate common_types;
extern crate ethereum_support;
extern crate futures;
extern crate hex;
extern crate memsocket;
extern crate pretty_env_logger;
extern crate secp256k1_support;
extern crate spectral;

use bitcoin_support::{BitcoinQuantity, Blocks};
use comit_node::swap_protocols::{
    json_config,
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003,
    wire_types::SwapResponse,
    SwapRequestHandler,
};
use common_types::seconds::Seconds;
use ethereum_support::EthereumQuantity;
use futures::future::Future;
use hex::FromHex;
use spectral::prelude::*;
use std::str::FromStr;
use tokio::runtime::Runtime;
use transport_protocol::{
    client::*,
    config::Config,
    connection::*,
    json::*,
    shutdown_handle::{self, ShutdownHandle},
    Status,
};

fn setup<
    H: SwapRequestHandler<
            rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
            rfc003::AcceptResponse<Bitcoin, Ethereum>,
        > + SwapRequestHandler<
            rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>,
            rfc003::AcceptResponse<Ethereum, Bitcoin>,
        >,
>(
    swap_request_handler: H,
) -> (
    Runtime,
    Client<Frame, Request, Response>,
    Client<Frame, Request, Response>,
    ShutdownHandle,
    ShutdownHandle,
) {
    let (alice, bob) = memsocket::unbounded();
    let mut runtime = Runtime::new().unwrap();

    let (alice_server, bob_client) =
        Connection::new(Config::default(), JsonFrameCodec::default(), alice)
            .start::<JsonFrameHandler>();
    let (alice_server, alice_shutdown_handle) = shutdown_handle::new(alice_server);

    let (bob_server, alice_client) = Connection::new(
        json_config(swap_request_handler),
        JsonFrameCodec::default(),
        bob,
    ).start::<JsonFrameHandler>();
    let (bob_server, bob_shutdown_handle) = shutdown_handle::new(bob_server);

    runtime.spawn(alice_server.map_err(|_| ()));
    runtime.spawn(bob_server.map_err(|_| ()));

    (
        runtime,
        alice_client,
        bob_client,
        alice_shutdown_handle,
        bob_shutdown_handle,
    )
}

#[derive(PartialEq)]
enum OfferDirection {
    BtcToEth,
    EthToBtc,
}

lazy_static! {
    static ref BTC_REFUND_PUBKEYHASH: bitcoin_support::PubkeyHash =
        bitcoin_support::PubkeyHash::from_hex("875638cac0b0ae9f826575e190f2788918c354c2").unwrap();
    static ref BTC_SUCCESS_PUBKEYHASH: bitcoin_support::PubkeyHash =
        bitcoin_support::PubkeyHash::from_hex("30bfdb95f68bfdd558a8dc6deef0da882b0c4866").unwrap();
    static ref ETH_REFUND_ADDRESS: ethereum_support::Address =
        ethereum_support::Address::from_str("8457037fcd80a8650c4692d7fcfc1d0a96b92867").unwrap();
    static ref ETH_SUCCESS_ADDRESS: ethereum_support::Address =
        ethereum_support::Address::from_str("0ae91a668e3ad094e765ec66f5d5c72e0b82f04d").unwrap();
}

fn gen_request(direction: OfferDirection) -> Request {
    let bitcoin = json!("Bitcoin");
    let ethereum = json!("Ethereum");
    let bitcoin_asset = json!({
            "value": "Bitcoin",
            "parameters": {
                "quantity": "100000000",
            }
    });
    let ethereum_asset = json!({
        "value": "Ether",
        "parameters": {
            "quantity": "10000000000000000000",
        }
    });

    let (
        source_ledger,
        target_ledger,
        source_asset,
        target_asset,
        source_ledger_refund_identity,
        target_ledger_success_identity,
    ) = match direction {
        OfferDirection::BtcToEth => (
            bitcoin,
            ethereum,
            bitcoin_asset,
            ethereum_asset,
            hex::encode(BTC_REFUND_PUBKEYHASH.clone()),
            format!("0x{}", hex::encode(ETH_SUCCESS_ADDRESS.clone())),
        ),
        OfferDirection::EthToBtc => (
            ethereum,
            bitcoin,
            ethereum_asset,
            bitcoin_asset,
            format!("0x{}", hex::encode(ETH_REFUND_ADDRESS.clone())),
            hex::encode(BTC_SUCCESS_PUBKEYHASH.clone()),
        ),
    };

    let headers = convert_args!(hashmap!(
        "source_ledger" => source_ledger,
        "target_ledger" => target_ledger,
        "source_asset" => source_asset,
        "target_asset" => target_asset,
        "swap_protocol" => json!("COMIT-RFC-003"),
    ));

    let body = json!({
        "source_ledger_refund_identity": source_ledger_refund_identity,
        "target_ledger_success_identity": target_ledger_success_identity,
        "source_ledger_lock_duration": 144,
        "secret_hash": "f6fc84c9f21c24907d6bee6eec38cabab5fa9a7be8c4a7827fe9e56f245bd2d5"
    });

    println!("{}", serde_json::to_string(&body).unwrap());

    Request::new("SWAP".into(), headers, body)
}

#[test]
fn can_receive_swap_request() {
    struct CaptureSwapMessage {
        sender: Option<
            futures::sync::oneshot::Sender<
                rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
            >,
        >,
    }

    impl
        SwapRequestHandler<
            rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
            rfc003::AcceptResponse<Bitcoin, Ethereum>,
        > for CaptureSwapMessage
    {
        fn handle(
            &mut self,
            request: rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
        ) -> SwapResponse<rfc003::AcceptResponse<Bitcoin, Ethereum>> {
            self.sender.take().unwrap().send(request).unwrap();
            SwapResponse::Decline
        }
    }

    impl
        SwapRequestHandler<
            rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>,
            rfc003::AcceptResponse<Ethereum, Bitcoin>,
        > for CaptureSwapMessage
    {}

    ::pretty_env_logger::try_init();

    let (sender, receiver) = futures::sync::oneshot::channel();

    let handler = CaptureSwapMessage {
        sender: Some(sender),
    };

    let (_runtime, _to_alice, mut to_bob, _alice_handle, _bob_handle) = setup(handler);

    let _response = to_bob
        .send_request(gen_request(OfferDirection::BtcToEth))
        .wait();

    assert_that(&_response)
        .is_ok()
        .map(|r| r.status())
        .is_equal_to(Status::SE(21));

    let result = receiver.wait();

    let expected_request = rfc003::Request {
        source_ledger: Bitcoin {},
        target_ledger: Ethereum {},
        source_asset: BitcoinQuantity::from_satoshi(100_000_000),
        target_asset: EthereumQuantity::from_eth(10.0),
        source_ledger_lock_duration: Blocks::from(144),
        source_ledger_refund_identity: BTC_REFUND_PUBKEYHASH.clone(),
        target_ledger_success_identity: ETH_SUCCESS_ADDRESS.clone(),
        secret_hash: "f6fc84c9f21c24907d6bee6eec38cabab5fa9a7be8c4a7827fe9e56f245bd2d5"
            .parse()
            .unwrap(),
    };

    assert_that(&result).is_ok().is_equal_to(&expected_request)
}

struct AcceptRate {
    pub btc_to_eth: f64,
}

impl
    SwapRequestHandler<
        rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
        rfc003::AcceptResponse<Bitcoin, Ethereum>,
    > for AcceptRate
{
    fn handle(
        &mut self,
        request: rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
    ) -> SwapResponse<rfc003::AcceptResponse<Bitcoin, Ethereum>> {
        let bitcoin = request.source_asset.bitcoin();
        let ethereum = request.target_asset.ethereum();

        let requested_rate = bitcoin / ethereum;
        if requested_rate > self.btc_to_eth {
            SwapResponse::Accept(rfc003::AcceptResponse {
                target_ledger_refund_identity: ETH_REFUND_ADDRESS.clone(),
                source_ledger_success_identity: BTC_SUCCESS_PUBKEYHASH.clone(),
                target_ledger_lock_duration: Seconds::new(4200),
            })
        } else {
            SwapResponse::Decline
        }
    }
}

impl
    SwapRequestHandler<
        rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>,
        rfc003::AcceptResponse<Ethereum, Bitcoin>,
    > for AcceptRate
{
    fn handle(
        &mut self,
        request: rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>,
    ) -> SwapResponse<rfc003::AcceptResponse<Ethereum, Bitcoin>> {
        let bitcoin = request.target_asset.bitcoin();
        let ethereum = request.source_asset.ethereum();

        let requested_rate = bitcoin / ethereum;
        if requested_rate < self.btc_to_eth {
            SwapResponse::Accept(rfc003::AcceptResponse {
                target_ledger_refund_identity: BTC_REFUND_PUBKEYHASH.clone(),
                source_ledger_success_identity: ETH_SUCCESS_ADDRESS.clone(),
                target_ledger_lock_duration: Blocks::from(144),
            })
        } else {
            SwapResponse::Decline
        }
    }
}

#[test]
fn rate_handler_reject_offer_btc_eth() {
    // The offer gives 1 BTC in exchange 10 ETH
    // But I am only willing to spend 5 ETH for a BTC
    // so REJECT
    let handler = AcceptRate {
        btc_to_eth: 1.0 / 5.0,
    };
    let (_runtime, _to_alice, mut to_bob, _alice_handle, _bob_handle) = setup(handler);
    let response = to_bob
        .send_request(gen_request(OfferDirection::BtcToEth))
        .wait();

    assert_that(&response)
        .is_ok()
        .map(|r| r.status())
        .is_equal_to(Status::SE(21));
}

#[test]
fn rate_handler_accept_offer_btc_eth() {
    // The offer gives 1 BTC in exchange 10 ETH
    // I am willing to give at most 11 ETH for 1 BTC
    // so ACCEPT
    let handler = AcceptRate {
        btc_to_eth: 1.0 / 11.0,
    };
    let (_runtime, _to_alice, mut to_bob, _alice_handle, _bob_handle) = setup(handler);
    let response = to_bob
        .send_request(gen_request(OfferDirection::BtcToEth))
        .wait();

    let correct_response_body = json!({
        "target_ledger_refund_identity" : format!("0x{}", hex::encode(ETH_REFUND_ADDRESS.clone())),
        "source_ledger_success_identity" : hex::encode(BTC_SUCCESS_PUBKEYHASH.clone()),
        "target_ledger_lock_duration" : 4200,
    });

    assert_that(&response)
        .is_ok()
        .is_equal_to(&Response::new(Status::OK(20)).with_body(correct_response_body));
}

#[test]
fn rate_handler_reject_offer_eth_btc() {
    // The offer gives 10 ETH in exchange for 1 BTC
    // I am willing to accept at least 11 ETH for a BTC
    // so REJECT
    let handler = AcceptRate {
        btc_to_eth: 1.0 / 11.0,
    };
    let (_runtime, _to_alice, mut to_bob, _alice_handle, _bob_handle) = setup(handler);
    let response = to_bob
        .send_request(gen_request(OfferDirection::EthToBtc))
        .wait();

    assert_that(&response)
        .is_ok()
        .map(|r| r.status())
        .is_equal_to(Status::SE(21));
}

#[test]
fn rate_handler_accept_offer_eth_btc() {
    // The offer gives 10 ETH for 1 BTC
    // I am willing to accept at least 5 ETH for a BTC
    // so ACCEPT
    let handler = AcceptRate {
        btc_to_eth: 1.0 / 5.0,
    };
    let (_runtime, _to_alice, mut to_bob, _alice_handle, _bob_handle) = setup(handler);
    let response = to_bob
        .send_request(gen_request(OfferDirection::EthToBtc))
        .wait();

    let correct_response_body = json!({
        "target_ledger_refund_identity" :  hex::encode(BTC_REFUND_PUBKEYHASH.clone()),
        "source_ledger_success_identity" : format!("0x{}", hex::encode(ETH_SUCCESS_ADDRESS.clone())),
        "target_ledger_lock_duration" : 144,
    });

    assert_that(&response)
        .is_ok()
        .is_equal_to(&Response::new(Status::OK(20)).with_body(correct_response_body));
}
