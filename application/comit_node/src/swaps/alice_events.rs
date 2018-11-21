use event_store::Event;
use std::marker::PhantomData;
use swap_protocols::rfc003::{Ledger, Secret};
use swaps::common::SwapId;

#[derive(Clone, Debug)]
pub struct SentSwapRequest<AL: Ledger, BL: Ledger, AA, BA> {
    pub alpha_ledger: AL,
    pub beta_ledger: BL,
    pub beta_asset: BA,
    pub alpha_asset: AA,
    pub secret: Secret,
    pub beta_ledger_success_identity: BL::Identity,
    pub alpha_ledger_refund_identity: AL::Identity,
    pub alpha_ledger_lock_duration: AL::LockDuration,
}

impl<
        AL: Ledger,
        BL: Ledger,
        AA: Clone + Send + Sync + 'static,
        BA: Clone + Send + Sync + 'static,
    > Event for SentSwapRequest<AL, BL, AA, BA>
{
    type Prev = ();
}

#[derive(Clone, Debug)]
pub struct SwapRequestAccepted<AL: Ledger, BL: Ledger, AA, BA> {
    pub beta_ledger_refund_identity: BL::Identity,
    pub alpha_ledger_success_identity: AL::Identity,
    pub beta_ledger_lock_duration: BL::LockDuration,
    phantom: PhantomData<(AA, BA)>,
}

impl<AL: Ledger, BL: Ledger, AA, BA> SwapRequestAccepted<AL, BL, AA, BA> {
    pub fn new(
        beta_ledger_refund_identity: BL::Identity,
        alpha_ledger_success_identity: AL::Identity,
        beta_ledger_lock_duration: BL::LockDuration,
    ) -> Self {
        SwapRequestAccepted {
            beta_ledger_refund_identity,
            alpha_ledger_success_identity,
            beta_ledger_lock_duration,
            phantom: PhantomData,
        }
    }
}

impl<
        AL: Ledger,
        BL: Ledger,
        AA: Clone + Send + Sync + 'static,
        BA: Clone + Send + Sync + 'static,
    > Event for SwapRequestAccepted<AL, BL, AA, BA>
{
    type Prev = SentSwapRequest<AL, BL, AA, BA>;
}
#[derive(Clone, Debug)]
pub struct SwapRequestRejected<AL: Ledger, BL: Ledger, AA, BA> {
    phantom: PhantomData<(AL, BL, AA, BA)>,
}

impl<
        AL: Ledger,
        BL: Ledger,
        AA: Clone + Send + Sync + 'static,
        BA: Clone + Send + Sync + 'static,
    > Event for SwapRequestRejected<AL, BL, AA, BA>
{
    type Prev = SentSwapRequest<AL, BL, AA, BA>;
}

#[allow(clippy::new_without_default_derive)]
impl<AL: Ledger, BL: Ledger, AA, BA> SwapRequestRejected<AL, BL, AA, BA> {
    pub fn new() -> Self {
        SwapRequestRejected {
            phantom: PhantomData,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AlphaFunded<AL: Ledger, BL: Ledger, AA, BA> {
    pub uid: SwapId,
    phantom: PhantomData<(AL, BL, AA, BA)>,
}

impl<AL: Ledger, BL: Ledger, AA, BA> AlphaFunded<AL, BL, AA, BA> {
    pub fn new(uid: SwapId) -> AlphaFunded<AL, BL, AA, BA> {
        AlphaFunded {
            uid,
            phantom: PhantomData,
        }
    }
}

impl<
        AL: Ledger,
        BL: Ledger,
        AA: Clone + Send + Sync + 'static,
        BA: Clone + Send + Sync + 'static,
    > Event for AlphaFunded<AL, BL, AA, BA>
{
    type Prev = SwapRequestAccepted<AL, BL, AA, BA>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BetaFunded<AL: Ledger, BL: Ledger, AA, BA> {
    pub address: BL::Address,
    phantom: PhantomData<(AL, AA, BA)>,
}

impl<AL: Ledger, BL: Ledger, AA, BA> BetaFunded<AL, BL, AA, BA> {
    pub fn new(address: BL::Address) -> BetaFunded<AL, BL, AA, BA> {
        BetaFunded {
            address,
            phantom: PhantomData,
        }
    }
}

impl<
        AL: Ledger,
        BL: Ledger,
        AA: Clone + Send + Sync + 'static,
        BA: Clone + Send + Sync + 'static,
    > Event for BetaFunded<AL, BL, AA, BA>
{
    type Prev = AlphaFunded<AL, BL, AA, BA>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BetaRedeemed<AL: Ledger, BL: Ledger, AA, BA> {
    phantom: PhantomData<(AL, BL, AA, BA)>,
}

impl<AL: Ledger, BL: Ledger, AA, BA> BetaRedeemed<AL, BL, AA, BA> {
    pub fn new() -> BetaRedeemed<AL, BL, AA, BA> {
        BetaRedeemed {
            phantom: PhantomData,
        }
    }
}

impl<
        AL: Ledger,
        BL: Ledger,
        AA: Clone + Send + Sync + 'static,
        BA: Clone + Send + Sync + 'static,
    > Event for BetaRedeemed<AL, BL, AA, BA>
{
    type Prev = BetaFunded<AL, BL, AA, BA>;
}
