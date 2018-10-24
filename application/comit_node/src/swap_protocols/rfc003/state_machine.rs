use super::AcceptResponse;
use comit_client::SwapResponseError;
use futures::{future::Either, Async, Future};
use state_machine_future::{RentToOwn, StateMachineFuture};
use std::{collections::HashMap, hash::Hash, sync::RwLock};
use swap_protocols::rfc003::{ledger::Ledger, messages::Request, secret::Secret};

#[derive(Debug, Clone, PartialEq)]
pub enum StateMachineError {
    SwapResponse(SwapResponseError),
}

// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#[allow(type_alias_bounds)]
pub mod events {
    use comit_client::SwapReject;
    use swap_protocols::rfc003::{
        ledger::Ledger, messages::AcceptResponse, state_machine::StateMachineError,
    };
    use tokio::{self, prelude::future::Either};

    type Future<I> = tokio::prelude::future::Future<Item = I, Error = StateMachineError> + Send;

    pub type Response<SL, TL> = Future<Result<AcceptResponse<SL, TL>, SwapReject>>;
    pub type Funded<L: Ledger> = Future<L::HtlcId>;
    pub type Refunded<L: Ledger> = Future<L::TxId>;
    pub type Redeemed<L: Ledger> = Future<L::TxId>;
    pub type SourceRefundedOrTargetFunded<SL: Ledger, TL: Ledger> =
        Future<Either<(SL::TxId, Box<Funded<TL>>), (TL::HtlcId, Box<Refunded<SL>>)>>;
    pub type RedeemedOrRefunded<L: Ledger> =
        Future<Either<(L::TxId, Box<Redeemed<L>>), (L::TxId, Box<Refunded<L>>)>>;

}

pub trait Futures<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone>: Send {
    fn send_request(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
    ) -> &mut Box<events::Response<SL, TL>>;

    fn source_htlc_funded(
        &mut self,
        start: &Start<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
    ) -> &mut Box<events::Funded<SL>>;

    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        start: &Start<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
        source_htlc_id: &SL::HtlcId,
    ) -> &mut Box<events::SourceRefundedOrTargetFunded<SL, TL>>;

    fn target_htlc_redeemed_or_refunded(
        &mut self,
        target_htlc_id: &TL::HtlcId,
    ) -> &mut Box<events::RedeemedOrRefunded<TL>>;

    fn source_htlc_redeemed_or_refunded(
        &mut self,
        source_htlc_id: &SL::HtlcId,
    ) -> &mut Box<events::RedeemedOrRefunded<SL>>;
}

pub struct Context<SL: Ledger, TL: Ledger, SA, TA> {
    pub futures: Box<Futures<SL, TL, SA, TA>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum SwapOutcome {
    Rejected,
    SourceRefunded,
    BothRefunded,
    BothRedeemed,
    SourceRedeemedTargetRefunded,
    SourceRefundedTargetRedeemed,
}

#[derive(StateMachineFuture)]
#[state_machine_future(context = "Context", derive(Clone))]
pub enum Swap<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone> {
    #[state_machine_future(start, transitions(Accepted, Final))]
    Start {
        source_identity: SL::HtlcIdentity,
        target_identity: TL::HtlcIdentity,
        source_ledger: SL,
        target_ledger: TL,
        source_asset: SA,
        target_asset: TA,
        source_ledger_lock_duration: SL::LockDuration,
        secret: Secret,
    },

    #[state_machine_future(transitions(SourceFunded))]
    Accepted {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
    },

    #[state_machine_future(transitions(BothFunded, Final))]
    SourceFunded {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        source_htlc_id: SL::HtlcId,
    },

    #[state_machine_future(transitions(
        SourceFundedTargetRefunded,
        SourceRefundedTargetFunded,
        SourceRedeemedTargetFunded,
        SourceFundedTargetRedeemed
    ))]
    BothFunded {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,
        source_htlc_id: SL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRefunded {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        source_htlc_id: SL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceRefundedTargetFunded {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceRedeemedTargetFunded {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRedeemed {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_redeemed_txid: TL::TxId,
        source_htlc_id: SL::HtlcId,
    },

    #[state_machine_future(ready)]
    Final(SwapOutcome),

    #[state_machine_future(error)]
    Error(StateMachineError),
}

impl<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone> PollSwap<SL, TL, SA, TA>
    for Swap<SL, TL, SA, TA>
{
    fn poll_start<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, Start<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterStart<SL, TL, SA, TA>>, StateMachineError> {
        let request = Request {
            source_asset: state.source_asset.clone(),
            target_asset: state.target_asset.clone(),
            source_ledger: state.source_ledger.clone(),
            target_ledger: state.target_ledger.clone(),
            source_ledger_refund_identity: state.source_identity.clone().into(),
            target_ledger_success_identity: state.target_identity.clone().into(),
            source_ledger_lock_duration: state.source_ledger_lock_duration.clone(),
            secret_hash: state.secret.hash(),
        };

        let response = try_ready!(context.futures.send_request(&request).poll());

        let state = state.take();

        match response {
            Ok(swap_accepted) => transition!(Accepted {
                start: state,
                response: swap_accepted,
            }),
            Err(rejected) => transition!(Final(SwapOutcome::Rejected)),
        }
    }

    fn poll_accepted<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, Accepted<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterAccepted<SL, TL, SA, TA>>, StateMachineError> {
        let source_htlc_id = try_ready!(
            context
                .futures
                .source_htlc_funded(&state.start, &state.response)
                .poll()
        );

        let state = state.take();

        transition!(SourceFunded {
            start: state.start,
            response: state.response,
            source_htlc_id,
        })
    }

    fn poll_source_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFunded<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterSourceFunded<SL, TL, SA, TA>>, StateMachineError> {
        match try_ready!(
            context
                .futures
                .source_htlc_refunded_target_htlc_funded(
                    &state.start,
                    &state.response,
                    &state.source_htlc_id
                ).poll()
        ) {
            Either::A((source_refunded_txid, target_htlc_funded_future)) => {
                transition!(Final(SwapOutcome::SourceRefunded))
            }
            Either::B((target_htlc_id, source_htlc_refunded_future)) => {
                let state = state.take();
                transition!(BothFunded {
                    start: state.start,
                    response: state.response,
                    source_htlc_id: state.source_htlc_id,
                    target_htlc_id,
                })
            }
        }
    }

    fn poll_both_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, BothFunded<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterBothFunded<SL, TL, SA, TA>>, StateMachineError> {
        if let Async::Ready(redeemed_or_refunded) = context
            .futures
            .source_htlc_redeemed_or_refunded(&state.source_htlc_id)
            .poll()?
        {
            let state = state.take();
            match redeemed_or_refunded {
                Either::A((source_redeemed_txid, _)) => transition!(SourceRedeemedTargetFunded {
                    start: state.start,
                    response: state.response,
                    target_htlc_id: state.target_htlc_id,
                }),
                Either::B((source_refunded_txid, _)) => transition!(SourceRefundedTargetFunded {
                    start: state.start,
                    response: state.response,
                    target_htlc_id: state.target_htlc_id,
                }),
            }
        }

        match try_ready!(
            context
                .futures
                .target_htlc_redeemed_or_refunded(&state.target_htlc_id)
                .poll()
        ) {
            Either::A((target_redeemed_txid, _)) => {
                let state = state.take();
                transition!(SourceFundedTargetRedeemed {
                    start: state.start,
                    response: state.response,
                    target_redeemed_txid,
                    source_htlc_id: state.source_htlc_id,
                })
            }
            Either::B((target_refunded_txid, _)) => {
                let state = state.take();
                transition!(SourceFundedTargetRefunded {
                    start: state.start,
                    response: state.response,
                    source_htlc_id: state.source_htlc_id,
                })
            }
        }
    }

    fn poll_source_funded_target_refunded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFundedTargetRefunded<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterSourceFundedTargetRefunded>, StateMachineError> {
        match try_ready!(
            context
                .futures
                .source_htlc_redeemed_or_refunded(&state.source_htlc_id)
                .poll()
        ) {
            Either::A((source_redeemed_txid, _)) => {
                transition!(Final(SwapOutcome::SourceRedeemedTargetRefunded))
            }
            Either::B((source_refunded_txid, _)) => transition!(Final(SwapOutcome::BothRefunded)),
        }
    }

    fn poll_source_refunded_target_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceRefundedTargetFunded<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterSourceRefundedTargetFunded>, StateMachineError> {
        match try_ready!(
            context
                .futures
                .target_htlc_redeemed_or_refunded(&state.target_htlc_id)
                .poll()
        ) {
            Either::A((target_redeemed_txid, _)) => {
                transition!(Final(SwapOutcome::SourceRefundedTargetRedeemed))
            }
            Either::B((target_refunded_txid, _)) => transition!(Final(SwapOutcome::BothRefunded)),
        }
    }

    fn poll_source_redeemed_target_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceRedeemedTargetFunded<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterSourceRedeemedTargetFunded>, StateMachineError> {
        match try_ready!(
            context
                .futures
                .target_htlc_redeemed_or_refunded(&state.target_htlc_id)
                .poll()
        ) {
            Either::A((target_redeemed_txid, _)) => transition!(Final(SwapOutcome::BothRedeemed)),
            Either::B((target_refunded_txid, _)) => {
                transition!(Final(SwapOutcome::SourceRedeemedTargetRefunded))
            }
        }
    }

    fn poll_source_funded_target_redeemed<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFundedTargetRedeemed<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterSourceFundedTargetRedeemed>, StateMachineError> {
        match try_ready!(
            context
                .futures
                .source_htlc_redeemed_or_refunded(&state.source_htlc_id)
                .poll()
        ) {
            Either::A((target_redeemed_txid, _)) => transition!(Final(SwapOutcome::BothRedeemed)),
            Either::B((target_refunded_txid, _)) => {
                transition!(Final(SwapOutcome::SourceRefundedTargetRedeemed))
            }
        }
    }
}

impl<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone> SwapFuture<SL, TL, SA, TA> {
    pub fn new(
        initial_state: SwapStates<SL, TL, SA, TA>,
        context: Context<SL, TL, SA, TA>,
    ) -> Self {
        SwapFuture(Some(initial_state), context)
    }
}

trait StateRepo<K, SL: Ledger, TL: Ledger, SA: Clone, TA: Clone>: Send + Sync {
    fn set(&self, id: K, state: SwapStates<SL, TL, SA, TA>);
    fn get(&self, id: &K) -> Option<SwapStates<SL, TL, SA, TA>>;
}

// pub struct InMemoryStateRepo<K: Hash + Eq, SL: Ledger, TL: Ledger, SA: Clone, TA: Clone> {
//     data: RwLock<HashMap<K, SwapStates<SL, TL, SA, TA>>>,
// }

// impl<K: Hash + Eq, SL: Ledger, TL: Ledger, SA: Clone, TA: Clone> Default
//     for InMemoryStateRepo<K, SL, TL, SA, TA>
// {
//     fn default() -> Self {
//         InMemoryStateRepo {
//             data: RwLock::new(HashMap::new()),
//         }
//     }
// }

// impl<
//         K: Hash + Eq + Send + Sync,
//         SL: Ledger,
//         TL: Ledger,
//         SA: Clone + Send + Sync,
//         TA: Send + Sync + Clone,
//     > StateRepo<K, SL, TL, SA, TA> for InMemoryStateRepo<K, SL, TL, SA, TA>
// {
//     fn set(&self, id: K, state: SwapStates<SL, TL, SA, TA>) {
//         let mut repo = self
//             .data
//             .write()
//             .expect("Other thread should not have panicked while having the lock");
//         repo.insert(id, state);
//     }

//     fn get(&self, id: &K) -> Option<SwapStates<SL, TL, SA, TA>> {
//         let repo = self
//             .data
//             .read()
//             .expect("Other thread should not have panicked while having the lock");
//         repo.get(id).map(Clone::clone)
//     }
// }

// #[cfg(test)]
// mod tests {

//     use super::*;
//     use bitcoin_support::{self, BitcoinQuantity, Blocks};
//     use ethereum_support::{self, EtherQuantity};
//     use hex::FromHex;
//     use std::str::FromStr;
//     use swap_protocols::ledger::{Bitcoin, Ethereum};

//     #[test]
//     fn given_a_state_store_it() {
//         let repo: InMemoryStateRepo<
//             String,
//             Bitcoin,
//             Ethereum,
//             BitcoinQuantity,
//             EtherQuantity,
//         > = InMemoryStateRepo::default();

//         let state = SwapStates::Sent(Start {
//             request: Request {
//                 secret_hash: "f6fc84c9f21c24907d6bee6eec38cabab5fa9a7be8c4a7827fe9e56f245bd2d5"
//                     .parse()
//                     .unwrap(),
//                 source_ledger_refund_identity: bitcoin_support::PubkeyHash::from_hex(
//                     "875638cac0b0ae9f826575e190f2788918c354c2",
//                 ).unwrap(),
//                 target_ledger_success_identity: ethereum_support::Address::from_str(
//                     "8457037fcd80a8650c4692d7fcfc1d0a96b92867",
//                 ).unwrap(),
//                 source_ledger_lock_duration: Blocks::from(144),
//                 target_asset: EtherQuantity::from_eth(10.0),
//                 source_asset: BitcoinQuantity::from_bitcoin(1.0),
//                 source_ledger: Bitcoin::regtest(),
//                 target_ledger: Ethereum::default(),
//             },
//         });
//     }
// }
