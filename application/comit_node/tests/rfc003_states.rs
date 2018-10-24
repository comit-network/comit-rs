extern crate bitcoin_support;
extern crate comit_node;
extern crate ethereum_support;
extern crate futures;
extern crate hex;
extern crate secp256k1_support;
extern crate tokio;
use bitcoin_support::{BitcoinQuantity, Blocks};
use comit_node::{
    comit_client::SwapReject,
    ledger_query_service::{
        fake_query_service::SimpleFakeLedgerQueryService, BitcoinQuery, LedgerQueryServiceApiClient,
    },
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            bitcoin,
            ethereum::Seconds,
            state_machine::{
                self, events, Context, Futures, Start, StateMachineError, SwapFuture, SwapOutcome,
                SwapStates,
            },
            AcceptResponse, Ledger, Request, Secret, SecretHash,
        },
        wire_types,
    },
};

use std::sync::Arc;

use ethereum_support::EtherQuantity;
use futures::{
    future::{self, Either},
    Future,
};
use hex::FromHex;
use std::str::FromStr;

pub trait LedgerEvents<L: Ledger> {
    fn htlc_funded(
        &mut self,
        lqs: &Arc<LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>>,
        success_identity: L::Identity,
        refund_identity: L::Identity,
        secret_hash: SecretHash,
        lock_duration: L::LockDuration,
        ledger: L,
    ) -> &mut Box<events::Funded<L>>;
}

#[derive(Default)]
struct BitcoinLedgerEvents {
    htlc_funded_event: Option<Box<events::Funded<Bitcoin>>>,
}

impl LedgerEvents<Bitcoin> for BitcoinLedgerEvents {
    fn htlc_funded(
        &mut self,
        lqs: &Arc<LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>>,
        success_identity: bitcoin_support::PubkeyHash,
        refund_identity: bitcoin_support::PubkeyHash,
        secret_hash: SecretHash,
        lock_duration: Blocks,
        ledger: Bitcoin,
    ) -> &mut Box<events::Funded<Bitcoin>> {
        let htlc = bitcoin::Htlc::new(
            success_identity,
            refund_identity,
            secret_hash,
            lock_duration.into(),
        );
        let query = BitcoinQuery {
            to_address: Some(htlc.compute_address(ledger.network())),
        };
        lqs.create(query);

        unimplemented!()
    }
}

struct TestFutures<SL: Ledger, TL: Ledger> {
    pub response: Box<events::Response<SL, TL>>,
    pub lqs: Arc<LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>>,
    pub source_ledger_events: Box<LedgerEvents<SL> + Send>,
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
        request: &Request<SL, TL, SA, TA>,
    ) -> &mut Box<events::Response<SL, TL>> {
        &mut self.response
    }

    fn source_htlc_funded(
        &mut self,
        start: &Start<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
    ) -> &mut Box<events::Funded<SL>> {
        self.source_ledger_events.htlc_funded(
            &self.lqs,
            response.source_ledger_success_identity.clone().into(),
            start.source_identity.clone().into(),
            start.secret.hash(),
            start.source_ledger_lock_duration.clone(),
            start.source_ledger.clone(),
        )
    }

    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        request: &Start<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
        source_htlc_id: &SL::HtlcId,
    ) -> &mut Box<events::SourceRefundedOrTargetFunded<SL, TL>> {
        unimplemented!()
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

#[test]
fn when_swap_is_rejected_go_to_final_reject() {
    let test_futures = TestFutures {
        response: Box::new(future::ok(Err(SwapReject::Rejected))),
        lqs: Arc::new(SimpleFakeLedgerQueryService {
            bitcoin_results: vec![],
            ethereum_results: vec![],
        }),
        source_ledger_events: Box::new(BitcoinLedgerEvents::default()),
    };
    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    let context = Context {
        futures: Box::new(test_futures),
    };

    let final_state_future = SwapFuture::new(
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
        }),
        context,
    );

    let final_state = runtime.block_on(final_state_future);

    assert_eq!(final_state, Ok(SwapOutcome::Rejected))
}

#[test]
fn final_state_source_refunded() {
    let test_futures = TestFutures {
        response: Box::new(future::ok(Ok(AcceptResponse {
            target_ledger_refund_identity: ethereum_support::Address::from_str(
                "71b9f69dcabb340a3fe229c3f94f1662ad85e5e8",
            ).unwrap(),
            source_ledger_success_identity: bitcoin_support::PubkeyHash::from_hex(
                "d38e554430c4035f2877a579a07a99886153f071",
            ).unwrap(),
            target_ledger_lock_duration: Seconds(42),
        }))),
        lqs: Arc::new(SimpleFakeLedgerQueryService {
            bitcoin_results: vec![],
            ethereum_results: vec![],
        }),
        source_ledger_events: Box::new(BitcoinLedgerEvents::default()),
    };

    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    let context = Context {
        futures: Box::new(test_futures),
    };

    let final_state_future = SwapFuture::new(
        SwapStates::Start(Start {
            source_identity: secp256k1_support::KeyPair::from_secret_key_hex(
                "18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725",
            ).unwrap(),
            target_identity: ethereum_support::Address::from_str(
                "8457037fcd80a8650c4692d7fcfc1d0a96b92867",
            ).unwrap(),
            source_ledger: Bitcoin::regtest(),
            target_ledger: Ethereum::default(),
            source_asset: BitcoinQuantity::from_bitcoin(1.0),
            target_asset: EtherQuantity::from_eth(10.0),
            source_ledger_lock_duration: Blocks::from(5),
            secret: Secret::from(*b"hello world, you are beautiful!!"),
        }),
        context,
    );

    let final_state = runtime.block_on(final_state_future);

    assert_eq!(final_state, Ok(SwapOutcome::SourceRefunded))
}
