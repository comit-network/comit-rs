#[macro_use]
extern crate log;
extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate comit_node;
extern crate ethereum_support;
extern crate futures;
extern crate hex;
extern crate secp256k1_support;
extern crate tokio;
extern crate tokio_timer;
use bitcoin_support::{BitcoinQuantity, Blocks, OutPoint, Sha256dHash};
use comit_node::{
    comit_client::SwapReject,
    ledger_query_service::{
        fake_query_service::SimpleFakeLedgerQueryService,
        fetch_transaction_stream::FetchTransactionStream, BitcoinQuery,
        LedgerQueryServiceApiClient,
    },
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            bitcoin,
            ethereum::Seconds,
            state_machine::{self, *},
            AcceptResponse, Ledger, Request, Secret, SecretHash,
        },
        wire_types,
    },
};

use ethereum_support::EtherQuantity;
use futures::{
    future::{self, Either},
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    Future, Stream,
};
use hex::FromHex;
use std::{
    str::FromStr,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};
use tokio_timer::Interval;

#[derive(Default)]
struct TestFutures<SL: Ledger, TL: Ledger> {
    pub response: Option<Box<events::Response<SL, TL>>>,
    pub source_htlc_funded: Option<Box<events::Funded<SL>>>,
    pub source_htlc_refunded_target_htlc_funded:
        Option<Box<events::SourceRefundedOrTargetFunded<SL, TL>>>,
}

impl<
        SL: Ledger,
        TL: Ledger,
        SA: Into<wire_types::Asset> + Clone,
        TA: Into<wire_types::Asset> + Clone,
    > Futures<SL, TL, SA, TA> for TestFutures<SL, TL>
{
    fn send_request(
        &mut self,
        _request: &Request<SL, TL, SA, TA>,
    ) -> &mut Box<events::Response<SL, TL>> {
        self.response.as_mut().unwrap()
    }

    fn source_htlc_funded(
        &mut self,
        start: &Start<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
    ) -> &mut Box<events::Funded<SL>> {
        self.source_htlc_funded.as_mut().unwrap()
    }

    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        request: &Start<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
        source_htlc_id: &SL::HtlcId,
    ) -> &mut Box<events::SourceRefundedOrTargetFunded<SL, TL>> {
        self.source_htlc_refunded_target_htlc_funded
            .as_mut()
            .unwrap()
    }

    fn target_htlc_redeemed_or_refunded(
        &mut self,
        target_htlc_id: &TL::HtlcId,
    ) -> &mut Box<events::RedeemedOrRefunded<TL>> {
        unimplemented!()
    }

    fn source_htlc_redeemed_or_refunded(
        &mut self,
        source_htlc_id: &SL::HtlcId,
    ) -> &mut Box<events::RedeemedOrRefunded<SL>> {
        unimplemented!()
    }
}

fn gen_start_state() -> SwapStates<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity> {
    SwapStates::Start(Start {
        source_identity: secp256k1_support::KeyPair::from_secret_key_slice(
            &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                .unwrap(),
        ).unwrap(),
        target_identity: ethereum_support::Address::from_str(
            "8457037fcd80a8650c4692d7fcfc1d0a96b92867",
        ).unwrap(),
        source_ledger: Bitcoin::regtest(),
        target_ledger: Ethereum::default(),
        source_asset: BitcoinQuantity::from_bitcoin(1.0),
        target_asset: EtherQuantity::from_eth(10.0),
        source_ledger_lock_duration: Blocks::from(144),
        secret: Secret::from(*b"hello world, you are beautiful!!"),
    })
}

fn init<
    SL: Ledger,
    TL: Ledger,
    SA: Clone + Send + Sync + Into<wire_types::Asset> + 'static,
    TA: Clone + Send + Sync + Into<wire_types::Asset> + 'static,
>(
    state: SwapStates<SL, TL, SA, TA>,
    test_futures: TestFutures<SL, TL>,
) -> (
    SwapFuture<SL, TL, SA, TA>,
    impl Stream<Item = SwapStates<SL, TL, SA, TA>, Error = ()>,
) {
    let (state_sender, state_receiver) = mpsc::unbounded();
    let context = Context {
        futures: Box::new(test_futures),
        state_repo: Arc::new(state_sender),
    };
    let final_state_future = SwapFuture::new(state, context);
    (final_state_future, state_receiver.map_err(|_| ()))
}

#[test]
fn when_swap_is_rejected_go_to_final_reject() {
    let start_state = gen_start_state();
    let test_futures = TestFutures {
        response: Some(Box::new(future::ok(Err(SwapReject::Rejected)))),
        ..Default::default()
    };
    let (final_state_future, state_stream) = init(start_state.clone(), test_futures);
    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    let outcome = runtime.block_on(final_state_future);
    let next_state = runtime.block_on(state_stream.take(1).collect()).unwrap();

    assert_eq!(outcome, Ok(SwapOutcome::Rejected));

    match next_state[0] {
        SwapStates::Final(ref final_outcome) => assert_eq!(final_outcome.0, outcome.unwrap()),
        _ => panic!("Didn't get to final state!"),
    };
}

#[test]
fn source_refunded() {
    let bob_response = AcceptResponse {
        target_ledger_refund_identity: ethereum_support::Address::from_str(
            "71b9f69dcabb340a3fe229c3f94f1662ad85e5e8",
        ).unwrap(),
        source_ledger_success_identity: bitcoin_support::PubkeyHash::from_hex(
            "d38e554430c4035f2877a579a07a99886153f071",
        ).unwrap(),
        target_ledger_lock_duration: Seconds(42),
    };
    let start_state = gen_start_state();
    let test_futures = TestFutures {
        response: Some(Box::new(future::ok(Ok(bob_response.clone())))),
        source_htlc_funded: Some(Box::new(future::ok(OutPoint {
            txid: Sha256dHash::from_data(b"funding"),
            vout: 0,
        }))),
        source_htlc_refunded_target_htlc_funded: Some(Box::new(future::ok(Either::A(
            Sha256dHash::from_data(b"refunded"),
        )))),
        ..Default::default()
    };
    let (final_state_future, state_stream) = init(start_state.clone(), test_futures);
    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    let outcome = runtime.block_on(final_state_future);
    let states = runtime.block_on(state_stream.take(3).collect()).unwrap();

    assert_eq!(outcome, Ok(SwapOutcome::SourceRefunded));

    match states[0] {
        SwapStates::Accepted(Accepted { ref response, .. }) => assert_eq!(response, &bob_response),
        _ => panic!("First state was not Accepted"),
    };

    match states[1] {
        SwapStates::SourceFunded(SourceFunded {
            ref source_htlc_id, ..
        }) => assert_eq!(
            source_htlc_id,
            &OutPoint {
                txid: Sha256dHash::from_data(b"funding"),
                vout: 0
            }
        ),
        _ => panic!("Second state wasn't SourceFunded"),
    }

    match states[2] {
        SwapStates::Final(ref final_outcome) => assert_eq!(final_outcome.0, outcome.unwrap()),
        _ => panic!("Third state wasn't Final"),
    }
}
