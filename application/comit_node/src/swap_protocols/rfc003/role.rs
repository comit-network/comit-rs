use crate::swap_protocols::{
    self,
    asset::Asset,
    rfc003::{ledger::Ledger, SecretHash},
};
use std::fmt::Debug;

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

pub trait Actions {
    type ActionKind;

    fn actions(&self) -> Vec<Self::ActionKind>;
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::{
        comit_client,
        swap_protocols::{
            ledger::{Bitcoin, Ethereum},
            rfc003::{
                events::{CommunicationEvents, ResponseFuture},
                Alice, Bob,
            },
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
