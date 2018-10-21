extern crate bitcoin_support;
extern crate comit_node;
extern crate common_types;
extern crate ethereum_support;
extern crate futures;
extern crate hex;
extern crate tokio;
use bitcoin_support::{BitcoinQuantity, Blocks};
use comit_node::{
    comit_client::{fake::FakeClient, Client, SwapReject},
    swap_protocols::{
        ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
        rfc003::{
            ledger::Ledger,
            state_machine::{self, events, Services, StateMachineError, SwapOutcome},
            AcceptResponse, Request,
        },
        wire_types,
    },
};
use common_types::secret::Secret;
use ethereum_support::EtherQuantity;
use futures::{future, Future};
use hex::FromHex;
use std::{marker::PhantomData, str::FromStr, sync::Arc};

#[derive(Default)]
struct TestServices {}

impl<
        SL: Ledger,
        TL: Ledger,
        SA: Into<wire_types::Asset> + Clone,
        TA: Into<wire_types::Asset> + Clone,
    > Services<SL, TL, SA, TA> for TestServices
{
    fn send_request(&self, request: &Request<SL, TL, SA, TA>) -> Box<events::Response<SL, TL>> {
        Box::new(future::ok(Err(SwapReject::Rejected)))
    }

    // fn send_request(&self, request: &Request<SL, TL, SA, TA>) -> Box<events::Response<SL, TL>> {
    //     Box::new(
    //         self.comit_client
    //             .send_swap_request(request.clone())
    //             .map_err(StateMachineError::SwapResponse),
    //     )
    // }

    fn source_htlc_funded(
        &self,
        request: &Request<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
    ) -> Box<events::Funded<SL>> {
        unimplemented!()
    }

    fn source_htlc_refunded(&self, source_htlc_id: &SL::HtlcId) -> Box<events::Refunded<SL>> {
        unimplemented!()
    }
    fn source_htlc_redeemed(&self, source_htlc_id: &SL::HtlcId) -> Box<events::Redeemed<SL>> {
        unimplemented!()
    }

    fn target_htlc_funded(
        &self,
        request: &Request<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
    ) -> Box<events::Funded<TL>> {
        unimplemented!()
    }

    fn target_htlc_refunded(&self, target_htlc_id: &TL::HtlcId) -> Box<events::Refunded<TL>> {
        unimplemented!()
    }

    fn target_htlc_redeemed(&self, target_htlc_id: &TL::HtlcId) -> Box<events::Redeemed<TL>> {
        unimplemented!()
    }
}

#[test]
fn when_swap_is_rejected_go_to_final_reject() {
    let test_services = TestServices::default();
    let mut runtime = tokio::runtime::Runtime::new().unwrap();

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
        Box::new(test_services),
        None,
    ));

    assert_eq!(final_state, Ok(SwapOutcome::Rejected))
}
