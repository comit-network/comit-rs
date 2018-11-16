use bitcoin_rpc_client::rpc::{SerializedRawTransaction, VerboseRawTransaction};
use bitcoin_support::{BitcoinQuantity, Blocks, OutPoint, Sha256dHash};
use comit_client::SwapReject;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        ethereum::Seconds,
        events::{
            self, Events, HtlcFunded, SourceHtlcRedeemedOrRefunded,
            SourceHtlcRefundedTargetHtlcFunded, TargetHtlcRedeemedOrRefunded,
        },
        roles::{
            test::{Alisha, Bobisha, FakeResponseSource},
            Bob, Role,
        },
        state_machine::*,
        Ledger, Secret,
    },
};

use ethereum_support::EtherQuantity;
use futures::{
    future::{self, Either},
    sync::mpsc,
    Stream,
};
use hex::FromHex;
use std::{str::FromStr, sync::Arc};

#[derive(Default)]
struct FakeEvents {
    pub source_htlc_funded: Option<Box<events::Funded<Bitcoin>>>,
    pub source_htlc_refunded_target_htlc_funded:
        Option<Box<events::SourceRefundedOrTargetFunded<Bitcoin, Ethereum>>>,
}

impl HtlcFunded<Bitcoin, BitcoinQuantity> for FakeEvents {
    fn htlc_funded(
        &mut self,
        _htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
    ) -> &mut events::Funded<Bitcoin> {
        self.source_htlc_funded.as_mut().unwrap()
    }
}

impl SourceHtlcRefundedTargetHtlcFunded<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>
    for FakeEvents
{
    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        _source_htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        _target_htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        _source_htlc_location: &bitcoin_support::OutPoint,
    ) -> &mut events::SourceRefundedOrTargetFunded<Bitcoin, Ethereum> {
        self.source_htlc_refunded_target_htlc_funded
            .as_mut()
            .unwrap()
    }
}

impl TargetHtlcRedeemedOrRefunded<Ethereum, EtherQuantity> for FakeEvents {
    fn target_htlc_redeemed_or_refunded(
        &mut self,
        _target_htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        _target_htlc_location: &ethereum_support::Address,
    ) -> &mut events::RedeemedOrRefunded<Ethereum> {
        unimplemented!()
    }
}

impl SourceHtlcRedeemedOrRefunded<Bitcoin, BitcoinQuantity> for FakeEvents {
    fn source_htlc_redeemed_or_refunded(
        &mut self,
        _source_htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        _target_htlc_location: &bitcoin_support::OutPoint,
    ) -> &mut events::RedeemedOrRefunded<Bitcoin> {
        unimplemented!()
    }
}

impl Events<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity> for FakeEvents {}

fn gen_start_state() -> Start<Alisha> {
    Start {
        source_ledger_refund_identity: secp256k1_support::KeyPair::from_secret_key_slice(
            &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                .unwrap(),
        )
        .unwrap(),
        target_ledger_success_identity: ethereum_support::Address::from_str(
            "8457037fcd80a8650c4692d7fcfc1d0a96b92867",
        )
        .unwrap(),
        source_ledger: Bitcoin::regtest(),
        target_ledger: Ethereum::default(),
        source_asset: BitcoinQuantity::from_bitcoin(1.0),
        target_asset: EtherQuantity::from_eth(10.0),
        source_ledger_lock_duration: Blocks::from(144),
        secret: Secret::from(*b"hello world, you are beautiful!!"),
    }
}

macro_rules! init {
    ($role:ty, $response_source:expr, $state:expr, $events:expr) => {{
        let (state_sender, state_receiver) = mpsc::unbounded();
        let context = Context {
            events: Box::new($events),
            state_repo: Arc::new(state_sender),
            response_source: Box::new($response_source),
        };
        let state: SwapStates<$role> = $state;
        let final_state_future = Swap::start_in(state, context);
        (final_state_future, state_receiver.map_err(|_| ()))
    }};
}

macro_rules! run_state_machine {
    ($state_machine:ident, $states:ident, $( $expected_state:expr ) , * ) => {
        {
            let mut expected_states = Vec::new();

            $(
                let state = $expected_state;
                expected_states.push(SwapStates::from(state));
            )
                *

                let number_of_expected_states = expected_states.len() + 1;

            let mut runtime = tokio::runtime::Runtime::new().unwrap();

            let state_machine_result = runtime.block_on($state_machine).unwrap();
            let actual_states = runtime.block_on($states.take(number_of_expected_states as u64).collect()).unwrap();

            expected_states.push(SwapStates::from(Final(state_machine_result)));

            assert_eq!(actual_states, expected_states);
        }
    };

    ($state_machine:ident, $states:ident) => {
        run_state_machine!($state_machine, $states, );
    };
}

#[test]
fn when_swap_is_rejected_go_to_final_reject() {
    let start = gen_start_state();

    let (state_machine, states) = init!(
        Alisha,
        FakeResponseSource::<Alisha> {
            response: Some(Box::new(future::ok(Err(SwapReject::Rejected)))),
        },
        start.clone().into(),
        FakeEvents {
            ..Default::default()
        }
    );

    run_state_machine!(state_machine, states);
}

#[test]
fn source_refunded() {
    let bob_response = StateMachineResponse {
        target_ledger_refund_identity: ethereum_support::Address::from_str(
            "71b9f69dcabb340a3fe229c3f94f1662ad85e5e8",
        )
        .unwrap(),
        source_ledger_success_identity: bitcoin_support::PubkeyHash::from_hex(
            "d38e554430c4035f2877a579a07a99886153f071",
        )
        .unwrap(),
        target_ledger_lock_duration: Seconds(42),
    };

    let start = gen_start_state();

    let (state_machine, states) = init!(
        Alisha,
        FakeResponseSource::<Alisha> {
            response: Some(Box::new(future::ok(Ok(bob_response.clone())))),
        },
        start.clone().into(),
        FakeEvents {
            source_htlc_funded: Some(Box::new(future::ok(OutPoint {
                txid: Sha256dHash::from_data(b"funding"),
                vout: 0,
            }))),
            source_htlc_refunded_target_htlc_funded: Some(Box::new(future::ok(Either::A(
                VerboseRawTransaction {
                    txid: Sha256dHash::from_data(b"refunded"),
                    hash: String::from(""),
                    size: 0,
                    vsize: 0,
                    version: 1,
                    locktime: 42,
                    vin: Vec::new(),
                    vout: Vec::new(),
                    hex: SerializedRawTransaction(String::from("")),
                    blockhash: Sha256dHash::from_data(b"blockhash"),
                    confirmations: 0,
                    time: 0,
                    blocktime: 0,
                }
                .into(),
            )))),
            ..Default::default()
        }
    );

    run_state_machine!(
        state_machine,
        states,
        Accepted {
            swap: OngoingSwap::new(start.clone(), bob_response.clone().into()),
        },
        SourceFunded {
            swap: OngoingSwap::new(start.clone(), bob_response.clone().into()),
            source_htlc_location: OutPoint {
                txid: Sha256dHash::from_data(b"funding"),
                vout: 0
            }
        }
    );
}

#[test]
fn bob_transition_source_refunded() {
    let start = Start {
        source_ledger_refund_identity: bitcoin_support::PubkeyHash::from_hex(
            "d38e554430c4035f2877a579a07a99886153f071",
        )
        .unwrap(),
        target_ledger_success_identity: ethereum_support::Address::from_str(
            "8457037fcd80a8650c4692d7fcfc1d0a96b92867",
        )
        .unwrap(),
        source_ledger: Bitcoin::regtest(),
        target_ledger: Ethereum::default(),
        source_asset: BitcoinQuantity::from_bitcoin(1.0),
        target_asset: EtherQuantity::from_eth(10.0),
        source_ledger_lock_duration: Blocks::from(144),
        secret: Secret::from(*b"hello world, you are beautiful!!").hash(),
    };

    let response = StateMachineResponse {
        source_ledger_success_identity: secp256k1_support::KeyPair::from_secret_key_slice(
            &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                .unwrap(),
        )
        .unwrap(),
        target_ledger_refund_identity: ethereum_support::Address::from_str(
            "8457037fcd80a8650c4692d7fcfc1d0a96b92867",
        )
        .unwrap(),
        target_ledger_lock_duration: Seconds(42),
    };

    let (state_machine, states) = init!(
        Bobisha,
        FakeResponseSource::<Bobisha> {
            response: Some(Box::new(future::ok(Ok(response.clone()))))
        },
        start.clone().into(),
        FakeEvents {
            source_htlc_funded: Some(Box::new(future::ok(OutPoint {
                txid: Sha256dHash::from_data(b"funding"),
                vout: 0,
            }))),
            source_htlc_refunded_target_htlc_funded: Some(Box::new(future::ok(Either::A(
                VerboseRawTransaction {
                    txid: Sha256dHash::from_data(b"refunded"),
                    hash: String::from(""),
                    size: 0,
                    vsize: 0,
                    version: 1,
                    locktime: 42,
                    vin: Vec::new(),
                    vout: Vec::new(),
                    hex: SerializedRawTransaction(String::from("")),
                    blockhash: Sha256dHash::from_data(b"blockhash"),
                    confirmations: 0,
                    time: 0,
                    blocktime: 0,
                }
                .into(),
            )))),
            ..Default::default()
        }
    );

    run_state_machine!(
        state_machine,
        states,
        Accepted {
            swap: OngoingSwap::new(start.clone(), response.clone().into())
        },
        SourceFunded {
            swap: OngoingSwap::new(start.clone(), response.clone().into()),
            source_htlc_location: OutPoint {
                txid: Sha256dHash::from_data(b"funding"),
                vout: 0
            }
        }
    );
}
