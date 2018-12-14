use crate::{
    comit_client::{self, ClientFactory, SwapReject},
    swap_protocols::{
        self,
        asset::Asset,
        rfc003::{
            self,
            actions::bob::{Accept, Decline},
            events::{AliceToBob, LedgerEvents, ResponseFuture},
            ledger::Ledger,
            save_state::SaveState,
            state_machine::{Context, Start, StateMachineResponse, Swap, SwapOutcome},
            Secret, SecretHash,
        },
    },
};
use futures::{future, sync::oneshot, Future};
use std::{
    fmt::Debug,
    marker::PhantomData,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

pub trait Role: Send + Sync + Debug + Clone + 'static {
    type AlphaLedger: Ledger;
    type BetaLedger: Ledger;
    type AlphaAsset: Asset;
    type BetaAsset: Asset;
    type AlphaRedeemHtlcIdentity: Send
        + Sync
        + Clone
        + Debug
        + PartialEq
        + Into<<Self::AlphaLedger as swap_protocols::Ledger>::Identity>;

    type AlphaRefundHtlcIdentity: Send
        + Sync
        + Clone
        + Debug
        + PartialEq
        + Into<<Self::AlphaLedger as swap_protocols::Ledger>::Identity>;

    type BetaRedeemHtlcIdentity: Send
        + Sync
        + Clone
        + Debug
        + PartialEq
        + Into<<Self::BetaLedger as swap_protocols::Ledger>::Identity>;

    type BetaRefundHtlcIdentity: Send
        + Sync
        + Clone
        + Debug
        + PartialEq
        + Into<<Self::BetaLedger as swap_protocols::Ledger>::Identity>;

    type Secret: Send + Sync + Clone + Into<SecretHash> + Debug + PartialEq;
}

#[derive(Clone, Debug)]
pub struct Alice<AL, BL, AA, BA> {
    phantom_data: PhantomData<(AL, BL, AA, BA)>,
}

impl<AL, BL, AA, BA> Default for Alice<AL, BL, AA, BA> {
    fn default() -> Self {
        Self {
            phantom_data: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct Initiation<R: Role> {
    pub alpha_ledger_refund_identity: R::AlphaRefundHtlcIdentity,
    pub beta_ledger_redeem_identity: R::BetaRedeemHtlcIdentity,
    pub alpha_ledger: R::AlphaLedger,
    pub beta_ledger: R::BetaLedger,
    pub alpha_asset: R::AlphaAsset,
    pub beta_asset: R::BetaAsset,
    pub alpha_ledger_lock_duration: <R::AlphaLedger as Ledger>::LockDuration,
    pub secret: R::Secret,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> Alice<AL, BL, AA, BA> {
    pub fn new_state_machine<C: comit_client::Client>(
        initiation: Initiation<Self>,
        alpha_ledger_events: Box<dyn LedgerEvents<AL, AA>>,
        beta_ledger_events: Box<dyn LedgerEvents<BL, BA>>,
        comit_client_factory: Arc<dyn ClientFactory<C>>,
        comit_node_addr: SocketAddr,
        save_state: Arc<dyn SaveState<Self>>,
    ) -> Box<dyn Future<Item = SwapOutcome<Alice<AL, BL, AA, BA>>, Error = rfc003::Error> + Send>
    {
        let start_state = Start {
            alpha_ledger: initiation.alpha_ledger,
            beta_ledger: initiation.beta_ledger,
            alpha_asset: initiation.alpha_asset,
            beta_asset: initiation.beta_asset,
            alpha_ledger_refund_identity: initiation.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: initiation.beta_ledger_redeem_identity,
            alpha_ledger_lock_duration: initiation.alpha_ledger_lock_duration,
            secret: initiation.secret,
            role: Alice::default(),
        };
        save_state.save(start_state.clone().into());
        let comit_client = match comit_client_factory.client_for(comit_node_addr) {
            Ok(comit_client) => comit_client,
            // This mess will go away with #319
            Err(e) => {
                return Box::new(future::err(rfc003::Error::Internal(format!("{:?}", e))));
            }
        };

        let context = Context {
            alpha_ledger_events,
            beta_ledger_events,
            communication_events: Box::new(AliceToBob::new(Arc::clone(&comit_client))),
            state_repo: save_state,
        };

        Box::new(Swap::start_in(start_state, context))
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> Role for Alice<AL, BL, AA, BA> {
    type AlphaLedger = AL;
    type BetaLedger = BL;
    type AlphaAsset = AA;
    type BetaAsset = BA;
    type AlphaRedeemHtlcIdentity = AL::Identity;
    type AlphaRefundHtlcIdentity = AL::HtlcIdentity;
    type BetaRedeemHtlcIdentity = BL::HtlcIdentity;
    type BetaRefundHtlcIdentity = BL::Identity;
    type Secret = Secret;
}

#[derive(Debug, Clone)]
pub struct Bob<AL: Ledger, BL: Ledger, AA, BA> {
    phantom_data: PhantomData<(AL, BL, AA, BA)>,
    #[allow(clippy::type_complexity)]
    response_sender: Arc<
        Mutex<
            Option<
                oneshot::Sender<
                    Result<
                        StateMachineResponse<AL::HtlcIdentity, BL::HtlcIdentity, BL::LockDuration>,
                        SwapReject,
                    >,
                >,
            >,
        >,
    >,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> Bob<AL, BL, AA, BA> {
    pub fn create() -> (Self, Box<ResponseFuture<Self>>) {
        let (sender, receiver) = oneshot::channel();
        (
            Bob {
                phantom_data: PhantomData,
                response_sender: Arc::new(Mutex::new(Some(sender))),
            },
            Box::new(
                receiver
                    .map_err(|_e| unreachable!("For now, it should be impossible for the sender to go out of scope before the receiver") ),
            ),
        )
    }

    pub fn accept_action(&self) -> Accept<AL, BL> {
        Accept::new(self.response_sender.clone())
    }

    pub fn decline_action(&self) -> Decline<AL, BL> {
        Decline::new(self.response_sender.clone())
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> Role for Bob<AL, BL, AA, BA> {
    type AlphaLedger = AL;
    type BetaLedger = BL;
    type AlphaAsset = AA;
    type BetaAsset = BA;
    type AlphaRedeemHtlcIdentity = AL::HtlcIdentity;
    type AlphaRefundHtlcIdentity = AL::Identity;
    type BetaRedeemHtlcIdentity = BL::Identity;
    type BetaRefundHtlcIdentity = BL::HtlcIdentity;
    type Secret = SecretHash;
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::{
        comit_client,
        swap_protocols::{
            ledger::{Bitcoin, Ethereum},
            rfc003::events::{CommunicationEvents, ResponseFuture},
        },
    };
    use bitcoin_support::BitcoinQuantity;
    use ethereum_support::EtherQuantity;

    pub type Alisha = Alice<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>;
    pub type Bobisha = Bob<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>;

    impl PartialEq<Alisha> for Alisha {
        fn eq(&self, _: &Alisha) -> bool {
            true
        }
    }

    impl PartialEq<Bobisha> for Bobisha {
        fn eq(&self, _: &Bobisha) -> bool {
            true
        }
    }

    #[allow(missing_debug_implementations)]
    pub struct FakeCommunicationEvents<R: Role> {
        pub response: Option<Box<ResponseFuture<R>>>,
    }

    impl<R: Role> CommunicationEvents<R> for FakeCommunicationEvents<R> {
        fn request_responded(
            &mut self,
            _request: &comit_client::rfc003::Request<
                R::AlphaLedger,
                R::BetaLedger,
                R::AlphaAsset,
                R::BetaAsset,
            >,
        ) -> &mut ResponseFuture<R> {
            self.response.as_mut().unwrap()
        }
    }

}
