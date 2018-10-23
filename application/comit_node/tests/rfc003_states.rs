extern crate bitcoin_support;
extern crate comit_node;
extern crate ethereum_support;
extern crate futures;
extern crate hex;
extern crate tokio;
use bitcoin_support::{BitcoinQuantity, Blocks};
use comit_node::{
    comit_client::SwapReject,
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            state_machine::{self, events, Context, Futures, StateMachineError, SwapOutcome},
            AcceptResponse, Ledger, Request, Secret,
        },
        wire_types,
    },
};

use ethereum_support::EtherQuantity;
use futures::{
    future::{self, Either},
    Future,
};
use hex::FromHex;
use std::str::FromStr;

#[derive(Default)]
struct TestFutures<SL: Ledger, TL: Ledger> {
    request: Option<Box<events::Response<SL, TL>>>,
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
        self.request
            .get_or_insert_with(|| Box::new(future::ok(Err(SwapReject::Rejected))))
    }

    fn source_htlc_funded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
    ) -> &mut Box<events::Funded<SL>> {
        unimplemented!()
    }

    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
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
    let test_futures = TestFutures::default();
    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    let context = Context {
        futures: Box::new(test_futures),
    };

    let final_state = runtime.block_on(state_machine::Swap::<
        Bitcoin,
        Ethereum,
        BitcoinQuantity,
        EtherQuantity,
    >::start(
        Request {
            secret_hash: "f6fc84c9f21c24907d6bee6eec38cabab5fa9a7be8c4a7827fe9e56f245bd2d5"
                .parse()
                .unwrap(),
            source_ledger_refund_identity: bitcoin_support::PubkeyHash::from_hex(
                "875638cac0b0ae9f826575e190f2788918c354c2",
            ).unwrap(),
            target_ledger_success_identity: ethereum_support::Address::from_str(
                "8457037fcd80a8650c4692d7fcfc1d0a96b92867",
            ).unwrap(),
            source_ledger_lock_duration: Blocks::from(144),
            target_asset: EtherQuantity::from_eth(10.0),
            source_asset: BitcoinQuantity::from_bitcoin(1.0),
            source_ledger: Bitcoin::regtest(),
            target_ledger: Ethereum::default(),
        },
        context,
    ));

    assert_eq!(final_state, Ok(SwapOutcome::Rejected))
}
