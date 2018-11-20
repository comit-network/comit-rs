use std::{fmt::Debug, marker::PhantomData};

use swap_protocols::{
    self,
    asset::Asset,
    rfc003::{ledger::Ledger, Secret, SecretHash},
};

pub trait Role: Send + Clone + 'static {
    type SourceLedger: Ledger;
    type TargetLedger: Ledger;
    type SourceAsset: Asset;
    type TargetAsset: Asset;
    type SourceSuccessHtlcIdentity: Send
        + Sync
        + Clone
        + Debug
        + PartialEq
        + Into<<Self::SourceLedger as swap_protocols::Ledger>::Identity>;

    type SourceRefundHtlcIdentity: Send
        + Sync
        + Clone
        + Debug
        + PartialEq
        + Into<<Self::SourceLedger as swap_protocols::Ledger>::Identity>;

    type TargetSuccessHtlcIdentity: Send
        + Sync
        + Clone
        + Debug
        + PartialEq
        + Into<<Self::TargetLedger as swap_protocols::Ledger>::Identity>;

    type TargetRefundHtlcIdentity: Send
        + Sync
        + Clone
        + Debug
        + PartialEq
        + Into<<Self::TargetLedger as swap_protocols::Ledger>::Identity>;

    type Secret: Send + Sync + Clone + Into<SecretHash> + Debug + PartialEq;
}

#[derive(Clone, Debug)]
pub struct Alice<SL, TL, SA, TA> {
    phantom_data: PhantomData<(SL, TL, SA, TA)>,
}

impl<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset> Role for Alice<SL, TL, SA, TA> {
    type SourceLedger = SL;
    type TargetLedger = TL;
    type SourceAsset = SA;
    type TargetAsset = TA;
    type SourceSuccessHtlcIdentity = SL::Identity;
    type SourceRefundHtlcIdentity = SL::HtlcIdentity;
    type TargetSuccessHtlcIdentity = TL::HtlcIdentity;
    type TargetRefundHtlcIdentity = TL::Identity;
    type Secret = Secret;
}

#[derive(Debug, Clone)]
pub struct Bob<SL, TL, SA, TA> {
    phantom_data: PhantomData<(SL, TL, SA, TA)>,
}

impl<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset> Role for Bob<SL, TL, SA, TA> {
    type SourceLedger = SL;
    type TargetLedger = TL;
    type SourceAsset = SA;
    type TargetAsset = TA;
    type SourceSuccessHtlcIdentity = SL::HtlcIdentity;
    type SourceRefundHtlcIdentity = SL::Identity;
    type TargetSuccessHtlcIdentity = TL::Identity;
    type TargetRefundHtlcIdentity = TL::HtlcIdentity;
    type Secret = SecretHash;
}

#[cfg(test)]
pub mod test {
    use super::*;
    use bitcoin_support::BitcoinQuantity;
    use comit_client;
    use ethereum_support::EtherQuantity;
    use swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::events::{CommunicationEvents, ResponseFuture},
    };

    pub type Alisha = Alice<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>;
    pub type Bobisha = Bob<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>;

    impl PartialEq<Alisha> for Alisha {
        fn eq(&self, _: &Alisha) -> bool {
            unreachable!(
                "Rust erroneously forces me to be PartialEq even though I'm never instantiated"
            )
        }
    }

    impl PartialEq<Bobisha> for Bobisha {
        fn eq(&self, _: &Bobisha) -> bool {
            unreachable!(
                "Rust erroneously forces me to be PartialEq even though I'm never instantiated"
            )
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
                R::SourceLedger,
                R::TargetLedger,
                R::SourceAsset,
                R::TargetAsset,
            >,
        ) -> &mut ResponseFuture<R> {
            self.response.as_mut().unwrap()
        }
    }

}
