use comit_client;
use futures::Future;
use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    sync::Arc,
};

use swap_protocols::{
    self,
    asset::Asset,
    rfc003::{
        self,
        ledger::Ledger,
        messages::Request,
        state_machine::{ResponseFuture, ResponseSource, StateMachineResponseFuture},
        Secret, SecretHash,
    },
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

#[allow(dead_code)] // TODO: Remove "allow" when used
struct AliceComitClient<C, SL: Ledger, TL: Ledger> {
    #[allow(clippy::type_complexity)]
    response_future:
        Option<Box<StateMachineResponseFuture<SL::Identity, TL::Identity, TL::LockDuration>>>,
    client: Arc<C>,
}

impl<C: comit_client::Client, SL: Ledger, TL: Ledger, SA: Asset, TA: Asset>
    ResponseSource<Alice<SL, TL, SA, TA>> for AliceComitClient<C, SL, TL>
{
    fn request_responded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
    ) -> &mut ResponseFuture<Alice<SL, TL, SA, TA>> {
        let client = Arc::clone(&self.client);
        self.response_future.get_or_insert_with(|| {
            Box::new(
                client
                    .send_swap_request(request.clone())
                    .map_err(rfc003::Error::SwapResponse)
                    .map(|result| result.map(Into::into)),
            )
        })
    }
}

pub struct Alice<SL: Ledger, TL: Ledger, SA, TA> {
    phantom_data: PhantomData<(SL, TL, SA, TA)>,
}

impl<SL: Ledger, TL: Ledger, SA, TA> Debug for Alice<SL, TL, SA, TA> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Alice")
    }
}

impl<SL: Ledger, TL: Ledger, SA, TA> Clone for Alice<SL, TL, SA, TA> {
    fn clone(&self) -> Alice<SL, TL, SA, TA> {
        unreachable!("Rust is requiring me to be clone erroneously")
    }
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
    use bitcoin_support::{self, BitcoinQuantity};
    use ethereum_support::{self, EtherQuantity};
    use swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::ethereum::Seconds,
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
    pub struct FakeResponseSource<R: Role> {
        pub response: Option<Box<ResponseFuture<R>>>,
    }

    impl<R: Role> ResponseSource<R> for FakeResponseSource<R> {
        fn request_responded(
            &mut self,
            _request: &Request<R::SourceLedger, R::TargetLedger, R::SourceAsset, R::TargetAsset>,
        ) -> &mut ResponseFuture<R> {
            self.response.as_mut().unwrap()
        }
    }

}
