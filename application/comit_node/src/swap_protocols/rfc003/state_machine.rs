use super::AcceptResponse;
use comit_client::SwapResponseError;
use futures::{future::Either, Async, Future};
use state_machine_future::{RentToOwn, StateMachineFuture};
use std::{collections::HashMap, hash::Hash, sync::RwLock};
use swap_protocols::rfc003::{ledger::Ledger, messages::Request};

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

pub trait Futures<SL: Ledger, TL: Ledger, SA, TA>: Send {
    fn send_request(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
    ) -> &mut Box<events::Response<SL, TL>>;

    fn source_htlc_funded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
    ) -> &mut Box<events::Funded<SL>>;

    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
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

impl<SL: Ledger, TL: Ledger, SA, TA> Clone for Context<SL, TL, SA, TA> {
    fn clone(&self) -> Self {
        unimplemented!()
    }
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
pub enum Swap<SL: Ledger, TL: Ledger, SA, TA> {
    // Start {
    //     sl_identity: SL::
    // }
    #[state_machine_future(start, transitions(Accepted, Final))]
    Sent { request: Request<SL, TL, SA, TA> },

    #[state_machine_future(transitions(SourceFunded))]
    Accepted {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
    },

    #[state_machine_future(transitions(BothFunded, Final))]
    SourceFunded {
        request: Request<SL, TL, SA, TA>,
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
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,
        source_htlc_id: SL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRefunded {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        source_htlc_id: SL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceRefundedTargetFunded {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceRedeemedTargetFunded {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRedeemed {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_redeemed_txid: TL::TxId,
        source_htlc_id: SL::HtlcId,
    },

    #[state_machine_future(ready)]
    Final(SwapOutcome),

    #[state_machine_future(error)]
    Error(StateMachineError),
}

impl<SL: Ledger, TL: Ledger, SA, TA> PollSwap<SL, TL, SA, TA> for Swap<SL, TL, SA, TA> {
    fn poll_sent<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, Sent<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterSent<SL, TL, SA, TA>>, StateMachineError> {
        let response = try_ready!(context.futures.send_request(&state.request).poll());

        let state = state.take();

        match response {
            Ok(swap_accepted) => transition!(Accepted {
                request: state.request,
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
                .source_htlc_funded(&state.request, &state.response)
                .poll()
        );

        let state = state.take();

        transition!(SourceFunded {
            request: state.request,
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
                    &state.request,
                    &state.response,
                    &state.source_htlc_id
                ).poll()
        ) {
            Either::A((source_refunded_txid, target_htlc_funded_future)) => {
                let state = state.take();
                transition!(Final(SwapOutcome::SourceRefunded))
            }
            Either::B((target_htlc_id, source_htlc_refunded_future)) => {
                let state = state.take();
                transition!(BothFunded {
                    request: state.request,
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
                    request: state.request,
                    response: state.response,
                    target_htlc_id: state.target_htlc_id,
                }),
                Either::B((source_refunded_txid, _)) => transition!(SourceRefundedTargetFunded {
                    request: state.request,
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
                    request: state.request,
                    response: state.response,
                    target_redeemed_txid,
                    source_htlc_id: state.source_htlc_id,
                })
            }
            Either::B((target_refunded_txid, _)) => {
                let state = state.take();
                transition!(SourceFundedTargetRefunded {
                    request: state.request,
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

trait StateRepo<K, SL: Ledger, TL: Ledger, SA, TA>: Send + Sync {
    fn set(&self, id: K, state: SwapStates<SL, TL, SA, TA>);
    fn get(&self, id: &K) -> Option<SwapStates<SL, TL, SA, TA>>;
}

pub struct InMemoryStateRepo<K: Hash + Eq, SL: Ledger, TL: Ledger, SA, TA> {
    data: RwLock<HashMap<K, SwapStates<SL, TL, SA, TA>>>,
}

impl<K: Hash + Eq, SL: Ledger, TL: Ledger, SA, TA> Default
    for InMemoryStateRepo<K, SL, TL, SA, TA>
{
    fn default() -> Self {
        InMemoryStateRepo {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl<
        K: Hash + Eq + Send + Sync,
        SL: Ledger,
        TL: Ledger,
        SA: Clone + Send + Sync,
        TA: Send + Sync + Clone,
    > StateRepo<K, SL, TL, SA, TA> for InMemoryStateRepo<K, SL, TL, SA, TA>
{
    fn set(&self, id: K, state: SwapStates<SL, TL, SA, TA>) {
        let mut repo = self
            .data
            .write()
            .expect("Other thread should not have panicked while having the lock");
        repo.insert(id, state);
    }

    fn get(&self, id: &K) -> Option<SwapStates<SL, TL, SA, TA>> {
        let repo = self
            .data
            .read()
            .expect("Other thread should not have panicked while having the lock");
        repo.get(id).map(Clone::clone)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use bitcoin_support::{self, BitcoinQuantity, Blocks};
    use ethereum_support::{self, EtherQuantity};
    use hex::FromHex;
    use std::str::FromStr;
    use swap_protocols::ledger::{Bitcoin, Ethereum};

    #[test]
    fn given_a_state_store_it() {
        let repo: InMemoryStateRepo<
            String,
            Bitcoin,
            Ethereum,
            BitcoinQuantity,
            EtherQuantity,
        > = InMemoryStateRepo::default();

        let state = SwapStates::Sent(Sent {
            request: Request {
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
        });
    }
}
