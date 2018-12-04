use bitcoin_support::{BitcoinQuantity, Blocks, OutPoint, Sha256dHash};
use comit_client::SwapReject;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        ethereum::Seconds,
        events::{self, LedgerEvents},
        roles::test::{Alisha, Bobisha, FakeCommunicationEvents},
        state_machine::*,
        RedeemTransaction, Secret,
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
use swap_protocols::rfc003::Ledger;

#[derive(Default)]
struct FakeLedgerEvents<L: Ledger> {
    pub htlc_deployed: Option<Box<events::Deployed<L>>>,
    pub htlc_funded: Option<Box<events::Funded<L>>>,
    pub htlc_redeemed_or_refunded: Option<Box<events::RedeemedOrRefunded<L>>>,
}

impl LedgerEvents<Bitcoin, BitcoinQuantity> for FakeLedgerEvents<Bitcoin> {
    fn htlc_deployed(
        &mut self,
        _htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
    ) -> &mut events::Deployed<Bitcoin> {
        self.htlc_deployed.as_mut().unwrap()
    }

    fn htlc_funded(
        &mut self,
        _htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        _htlc_location: &bitcoin_support::OutPoint,
    ) -> &mut events::Funded<Bitcoin> {
        self.htlc_funded.as_mut().unwrap()
    }

    fn htlc_redeemed_or_refunded(
        &mut self,
        _htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        _htlc_location: &bitcoin_support::OutPoint,
    ) -> &mut events::RedeemedOrRefunded<Bitcoin> {
        self.htlc_redeemed_or_refunded.as_mut().unwrap()
    }
}

impl LedgerEvents<Ethereum, EtherQuantity> for FakeLedgerEvents<Ethereum> {
    fn htlc_deployed(
        &mut self,
        _htlc_params: HtlcParams<Ethereum, EtherQuantity>,
    ) -> &mut events::Deployed<Ethereum> {
        self.htlc_deployed.as_mut().unwrap()
    }

    fn htlc_funded(
        &mut self,
        _htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        _htlc_location: &ethereum_support::Address,
    ) -> &mut events::Funded<Ethereum> {
        unimplemented!()
    }

    fn htlc_redeemed_or_refunded(
        &mut self,
        _htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        _htlc_location: &ethereum_support::Address,
    ) -> &mut events::RedeemedOrRefunded<Ethereum> {
        unimplemented!()
    }
}

fn gen_start_state() -> Start<Alisha> {
    Start {
        alpha_ledger_refund_identity: secp256k1_support::KeyPair::from_secret_key_slice(
            &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                .unwrap(),
        )
        .unwrap(),
        beta_ledger_success_identity: ethereum_support::Address::from_str(
            "8457037fcd80a8650c4692d7fcfc1d0a96b92867",
        )
        .unwrap(),
        alpha_ledger: Bitcoin::regtest(),
        beta_ledger: Ethereum::default(),
        alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
        beta_asset: EtherQuantity::from_eth(10.0),
        alpha_ledger_lock_duration: Blocks::from(144),
        secret: Secret::from(*b"hello world, you are beautiful!!"),
        role: Alisha::new(),
    }
}

macro_rules! init {
    ($role:ty, $response_event:expr, $state:expr, $alpha_events:expr, $beta_events:expr) => {{
        let (state_sender, state_receiver) = mpsc::unbounded();
        let context = Context {
            alpha_ledger_events: Box::new($alpha_events),
            beta_ledger_events: Box::new($beta_events),
            state_repo: Arc::new(state_sender),
            communication_events: Box::new($response_event),
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
        FakeCommunicationEvents::<Alisha> {
            response: Some(Box::new(future::ok(Err(SwapReject::Rejected)))),
        },
        start.clone().into(),
        FakeLedgerEvents {
            ..Default::default()
        },
        FakeLedgerEvents {
            ..Default::default()
        }
    );

    run_state_machine!(state_machine, states);
}

#[test]
fn alpha_refunded() {
    let bob_response = StateMachineResponse {
        beta_ledger_refund_identity: ethereum_support::Address::from_str(
            "71b9f69dcabb340a3fe229c3f94f1662ad85e5e8",
        )
        .unwrap(),
        alpha_ledger_success_identity: bitcoin_support::PubkeyHash::from_hex(
            "d38e554430c4035f2877a579a07a99886153f071",
        )
        .unwrap(),
        beta_ledger_lock_duration: Seconds(42),
    };

    let start = gen_start_state();

    let (state_machine, states) = init!(
        Alisha,
        FakeCommunicationEvents::<Alisha> {
            response: Some(Box::new(future::ok(Ok(bob_response.clone())))),
        },
        start.clone().into(),
        FakeLedgerEvents::<Bitcoin> {
            htlc_deployed: Some(Box::new(future::ok(OutPoint {
                txid: Sha256dHash::from_data(b"funding"),
                vout: 0,
            }))),
            htlc_funded: Some(Box::new(future::ok(None))),
            htlc_redeemed_or_refunded: Some(Box::new(future::ok(Either::A(RedeemTransaction {
                transaction: bitcoin_support::Transaction {
                    version: 1,
                    lock_time: 42,
                    input: vec![],
                    output: vec![],
                },
                secret: start.secret,
            })))),
            ..Default::default()
        },
        FakeLedgerEvents::<Ethereum> {
            ..Default::default()
        }
    );

    run_state_machine!(
        state_machine,
        states,
        Accepted {
            swap: OngoingSwap::new(start.clone(), bob_response.clone().into()),
        },
        AlphaDeployed {
            swap: OngoingSwap::new(start.clone(), bob_response.clone().into()),
            alpha_htlc_location: OutPoint {
                txid: Sha256dHash::from_data(b"funding"),
                vout: 0
            }
        },
        AlphaFunded {
            swap: OngoingSwap::new(start.clone(), bob_response.clone().into()),
            alpha_htlc_location: OutPoint {
                txid: Sha256dHash::from_data(b"funding"),
                vout: 0
            }
        }
    );
}

#[test]
fn bob_transition_alpha_refunded() {
    let (bobisha, _) = Bobisha::new();
    let start = Start {
        alpha_ledger_refund_identity: bitcoin_support::PubkeyHash::from_hex(
            "d38e554430c4035f2877a579a07a99886153f071",
        )
        .unwrap(),
        beta_ledger_success_identity: ethereum_support::Address::from_str(
            "8457037fcd80a8650c4692d7fcfc1d0a96b92867",
        )
        .unwrap(),
        alpha_ledger: Bitcoin::regtest(),
        beta_ledger: Ethereum::default(),
        alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
        beta_asset: EtherQuantity::from_eth(10.0),
        alpha_ledger_lock_duration: Blocks::from(144),
        secret: Secret::from(*b"hello world, you are beautiful!!").hash(),
        role: bobisha,
    };

    let response = StateMachineResponse {
        alpha_ledger_success_identity: secp256k1_support::KeyPair::from_secret_key_slice(
            &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                .unwrap(),
        )
        .unwrap(),
        beta_ledger_refund_identity: ethereum_support::Address::from_str(
            "8457037fcd80a8650c4692d7fcfc1d0a96b92867",
        )
        .unwrap(),
        beta_ledger_lock_duration: Seconds(42),
    };

    let (state_machine, states) = init!(
        Bobisha,
        FakeCommunicationEvents::<Bobisha> {
            response: Some(Box::new(future::ok(Ok(response.clone()))))
        },
        start.clone().into(),
        FakeLedgerEvents::<Bitcoin> {
            htlc_deployed: Some(Box::new(future::ok(OutPoint {
                txid: Sha256dHash::from_data(b"funding"),
                vout: 0,
            }))),
            htlc_funded: Some(Box::new(future::ok(None))),
            htlc_redeemed_or_refunded: Some(Box::new(future::ok(Either::A(RedeemTransaction {
                transaction: bitcoin_support::Transaction {
                    version: 1,
                    lock_time: 42,
                    input: vec![],
                    output: vec![],
                },
                secret: Secret::from(*b"hello world, you are beautiful!!"),
            })))),
            ..Default::default()
        },
        FakeLedgerEvents::<Ethereum> {
            ..Default::default()
        }
    );

    run_state_machine!(
        state_machine,
        states,
        Accepted {
            swap: OngoingSwap::new(start.clone(), response.clone().into())
        },
        AlphaDeployed {
            swap: OngoingSwap::new(start.clone(), response.clone().into()),
            alpha_htlc_location: OutPoint {
                txid: Sha256dHash::from_data(b"funding"),
                vout: 0
            }
        },
        AlphaFunded {
            swap: OngoingSwap::new(start.clone(), response.clone().into()),
            alpha_htlc_location: OutPoint {
                txid: Sha256dHash::from_data(b"funding"),
                vout: 0
            }
        }
    );
}
