use crate::{
    db::AcceptedSwap,
    swap_protocols::{
        rfc003::{
            self,
            events::{
                Deployed, Funded, HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded, Redeemed,
                Refunded,
            },
            state_store::StateStore,
            Accept, ActorState, Request, SecretHash,
        },
        HashFunction,
    },
    timestamp::Timestamp,
};
use chrono::NaiveDateTime;
use futures_core::future::{self, Either};
use genawaiter::{
    sync::{Co, Gen},
    GeneratorState,
};

/// Returns a future that tracks the swap negotiated from the given request and
/// accept response on alpha ledger.
///
/// The current implementation is naive in the sense that it does not take into
/// account situations where it is clear that no more events will happen even
/// though in theory, there could. For example:
/// - alpha funded
/// - alpha refunded
///
/// It is highly unlikely for Bob to fund the HTLC now, yet the current
/// implementation is still waiting for that.
pub async fn create_alpha_watcher<D, A, AI, BI>(
    dependencies: D,
    accepted: AcceptedSwap<A::AL, A::BL, A::AA, A::BA, AI, BI>,
) where
    D: StateStore
        + HtlcFunded<A::AL, A::AA, A::AH, AI, A::AT>
        + HtlcFunded<A::BL, A::BA, A::BH, BI, A::BT>
        + HtlcDeployed<A::AL, A::AA, A::AH, AI, A::AT>
        + HtlcDeployed<A::BL, A::BA, A::BH, BI, A::BT>
        + HtlcRedeemed<A::AL, A::AA, A::AH, AI, A::AT>
        + HtlcRedeemed<A::BL, A::BA, A::BH, BI, A::BT>
        + HtlcRefunded<A::AL, A::AA, A::AH, AI, A::AT>
        + HtlcRefunded<A::BL, A::BA, A::BH, BI, A::BT>
        + Clone,
    A::AL: Clone,
    A::BL: Clone,
    A::AA: Ord + Clone,
    A::BA: Ord + Clone,
    A::AH: Clone,
    A::BH: Clone,
    AI: Clone,
    BI: Clone,
    A::AT: Clone,
    A::BT: Clone,
    A: ActorState,
    AcceptedSwap<A::AL, A::BL, A::AA, A::BA, AI, BI>: Clone,
{
    let (request, accept, at) = accepted;

    let id = request.swap_id;
    let swap = OngoingSwap::new(request, accept);

    // construct a generator that watches alpha and beta ledger concurrently
    let mut generator = Gen::new({
        let dependencies = dependencies.clone();
        |co| async move {
            watch_alpha_ledger::<_, A::AL, A::BL, A::AA, A::BA, A::AH, A::BH, AI, BI, A::AT, A::BT>(
                &dependencies,
                &co,
                swap.alpha_htlc_params(),
                at,
            )
            .await
        }
    });

    loop {
        // wait for events to be emitted as the generator executes
        match generator.async_resume().await {
            // every event that is yielded is passed on
            GeneratorState::Yielded(event) => {
                tracing::info!("swap {} yielded event {}", id, event);
                dependencies.update::<A>(&id, event);
            }
            // the generator stopped executing, this means there are no more events that can be
            // watched.
            GeneratorState::Complete(Ok(_)) => {
                tracing::info!("swap {} finished", id);
                return;
            }
            GeneratorState::Complete(Err(e)) => {
                tracing::error!("swap {} failed with {:?}", id, e);
                return;
            }
        }
    }
}

/// Returns a future that tracks the swap negotiated from the given request and
/// accept response on beta ledger.
///
/// The current implementation is naive in the sense that it does not take into
/// account situations where it is clear that no more events will happen even
/// though in theory, there could. For example:
/// - alpha funded
/// - alpha refunded
///
/// It is highly unlikely for Bob to fund the HTLC now, yet the current
/// implementation is still waiting for that.
pub async fn create_beta_watcher<D, A, AI, BI>(
    dependencies: D,
    accepted: AcceptedSwap<A::AL, A::BL, A::AA, A::BA, AI, BI>,
) where
    D: StateStore
        + HtlcFunded<A::AL, A::AA, A::AH, AI, A::AT>
        + HtlcFunded<A::BL, A::BA, A::BH, BI, A::BT>
        + HtlcDeployed<A::AL, A::AA, A::AH, AI, A::AT>
        + HtlcDeployed<A::BL, A::BA, A::BH, BI, A::BT>
        + HtlcRedeemed<A::AL, A::AA, A::AH, AI, A::AT>
        + HtlcRedeemed<A::BL, A::BA, A::BH, BI, A::BT>
        + HtlcRefunded<A::AL, A::AA, A::AH, AI, A::AT>
        + HtlcRefunded<A::BL, A::BA, A::BH, BI, A::BT>
        + Clone,
    A::AL: Clone,
    A::BL: Clone,
    A::AA: Ord + Clone,
    A::BA: Ord + Clone,
    A::AH: Clone,
    A::BH: Clone,
    A::AT: Clone,
    A::BT: Clone,
    AI: Clone,
    BI: Clone,
    A: ActorState,
    AcceptedSwap<A::AL, A::BL, A::AA, A::BA, AI, BI>: Clone,
{
    let (request, accept, at) = accepted.clone();

    let id = request.swap_id;
    let swap = OngoingSwap::new(request, accept);

    // construct a generator that watches alpha and beta ledger concurrently
    let mut generator = Gen::new({
        let dependencies = dependencies.clone();
        |co| async move {
            watch_beta_ledger::<_, A::AL, A::BL, A::AA, A::BA, A::AH, A::BH, AI, BI, A::AT, A::BT>(
                &dependencies,
                &co,
                swap.beta_htlc_params(),
                at,
            )
            .await
        }
    });

    loop {
        // wait for events to be emitted as the generator executes
        match generator.async_resume().await {
            // every event that is yielded is passed on
            GeneratorState::Yielded(event) => {
                tracing::info!("swap {} yielded event {}", id, event);
                dependencies.update::<A>(&id, event);
            }
            // the generator stopped executing, this means there are no more events that can be
            // watched.
            GeneratorState::Complete(Ok(_)) => {
                tracing::info!("swap {} finished", id);
                return;
            }
            GeneratorState::Complete(Err(e)) => {
                tracing::error!("swap {} failed with {:?}", id, e);
                return;
            }
        }
    }
}

/// Returns a future that waits for events on alpha ledger to happen.
///
/// Each event is yielded through the controller handle (co) of the coroutine.
async fn watch_alpha_ledger<D, AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>(
    dependencies: &D,
    co: &Co<SwapEvent<AA, BA, AH, BH, AT, BT>>,
    htlc_params: HtlcParams<AL, AA, AI>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    D: HtlcFunded<AL, AA, AH, AI, AT>
        + HtlcDeployed<AL, AA, AH, AI, AT>
        + HtlcRedeemed<AL, AA, AH, AI, AT>
        + HtlcRefunded<AL, AA, AH, AI, AT>,
    Deployed<AH, AT>: Clone,
    Redeemed<AT>: Clone,
    Refunded<AT>: Clone,
{
    let deployed = dependencies
        .htlc_deployed(&htlc_params, start_of_swap)
        .await?;
    co.yield_(SwapEvent::AlphaDeployed(deployed.clone())).await;

    let funded = dependencies
        .htlc_funded(&htlc_params, &deployed, start_of_swap)
        .await?;
    co.yield_(SwapEvent::AlphaFunded(funded)).await;

    let redeemed = dependencies.htlc_redeemed(&htlc_params, &deployed, start_of_swap);

    let refunded = dependencies.htlc_refunded(&htlc_params, &deployed, start_of_swap);

    match future::try_select(redeemed, refunded).await {
        Ok(Either::Left((redeemed, _))) => {
            co.yield_(SwapEvent::AlphaRedeemed(redeemed.clone())).await;
        }
        Ok(Either::Right((refunded, _))) => {
            co.yield_(SwapEvent::AlphaRefunded(refunded.clone())).await;
        }
        Err(either) => {
            let (error, _other_future) = either.factor_first();

            return Err(error);
        }
    }

    Ok(())
}

/// Returns a future that waits for events on beta ledger to happen.
///
/// Each event is yielded through the controller handle (co) of the coroutine.
async fn watch_beta_ledger<D, AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>(
    dependencies: &D,
    co: &Co<SwapEvent<AA, BA, AH, BH, AT, BT>>,
    htlc_params: HtlcParams<BL, BA, BI>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    D: HtlcFunded<BL, BA, BH, BI, BT>
        + HtlcDeployed<BL, BA, BH, BI, BT>
        + HtlcRedeemed<BL, BA, BH, BI, BT>
        + HtlcRefunded<BL, BA, BH, BI, BT>,
    Deployed<BH, BT>: Clone,
    Redeemed<BT>: Clone,
    Refunded<BT>: Clone,
{
    let deployed = dependencies
        .htlc_deployed(&htlc_params, start_of_swap)
        .await?;
    co.yield_(SwapEvent::BetaDeployed(deployed.clone())).await;

    let funded = dependencies
        .htlc_funded(&htlc_params, &deployed, start_of_swap)
        .await?;
    co.yield_(SwapEvent::BetaFunded(funded)).await;

    let redeemed = dependencies.htlc_redeemed(&htlc_params, &deployed, start_of_swap);

    let refunded = dependencies.htlc_refunded(&htlc_params, &deployed, start_of_swap);

    match future::try_select(redeemed, refunded).await {
        Ok(Either::Left((redeemed, _))) => {
            co.yield_(SwapEvent::BetaRedeemed(redeemed.clone())).await;
        }
        Ok(Either::Right((refunded, _))) => {
            co.yield_(SwapEvent::BetaRefunded(refunded.clone())).await;
        }
        Err(either) => {
            let (error, _other_future) = either.factor_first();

            return Err(error);
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct HtlcParams<L, A, I> {
    pub asset: A,
    pub ledger: L,
    pub redeem_identity: I,
    pub refund_identity: I,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl<L, A, I> HtlcParams<L, A, I>
where
    L: Clone,
    A: Clone,
    I: Clone,
{
    pub fn new_alpha_params<BL, BA, BI>(
        request: &rfc003::Request<L, BL, A, BA, I, BI>,
        accept_response: &rfc003::Accept<I, BI>,
    ) -> Self {
        HtlcParams {
            asset: request.alpha_asset.clone(),
            ledger: request.alpha_ledger.clone(),
            redeem_identity: accept_response.alpha_ledger_redeem_identity.clone(),
            refund_identity: request.alpha_ledger_refund_identity.clone(),
            expiry: request.alpha_expiry,
            secret_hash: request.secret_hash,
        }
    }

    pub fn new_beta_params<AL, AA, AI>(
        request: &rfc003::Request<AL, L, AA, A, AI, I>,
        accept_response: &rfc003::Accept<AI, I>,
    ) -> Self {
        HtlcParams {
            asset: request.beta_asset.clone(),
            ledger: request.beta_ledger.clone(),
            redeem_identity: request.beta_ledger_redeem_identity.clone(),
            refund_identity: accept_response.beta_ledger_refund_identity.clone(),
            expiry: request.beta_expiry,
            secret_hash: request.secret_hash,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OngoingSwap<AL, BL, AA, BA, AI, BI> {
    pub alpha_ledger: AL,
    pub beta_ledger: BL,
    pub alpha_asset: AA,
    pub beta_asset: BA,
    pub hash_function: HashFunction,
    pub alpha_ledger_redeem_identity: AI,
    pub alpha_ledger_refund_identity: AI,
    pub beta_ledger_redeem_identity: BI,
    pub beta_ledger_refund_identity: BI,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl<AL, BL, AA, BA, AI, BI> OngoingSwap<AL, BL, AA, BA, AI, BI> {
    pub fn new(request: Request<AL, BL, AA, BA, AI, BI>, accept: Accept<AI, BI>) -> Self {
        OngoingSwap {
            alpha_ledger: request.alpha_ledger,
            beta_ledger: request.beta_ledger,
            alpha_asset: request.alpha_asset,
            beta_asset: request.beta_asset,
            hash_function: request.hash_function,
            alpha_ledger_redeem_identity: accept.alpha_ledger_redeem_identity,
            alpha_ledger_refund_identity: request.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: request.beta_ledger_redeem_identity,
            beta_ledger_refund_identity: accept.beta_ledger_refund_identity,
            alpha_expiry: request.alpha_expiry,
            beta_expiry: request.beta_expiry,
            secret_hash: request.secret_hash,
        }
    }
}

impl<AL, BL, AA, BA, AI, BI> OngoingSwap<AL, BL, AA, BA, AI, BI>
where
    AL: Clone,
    AA: Clone,
    AI: Clone,
{
    pub fn alpha_htlc_params(&self) -> HtlcParams<AL, AA, AI> {
        HtlcParams {
            asset: self.alpha_asset.clone(),
            ledger: self.alpha_ledger.clone(),
            redeem_identity: self.alpha_ledger_redeem_identity.clone(),
            refund_identity: self.alpha_ledger_refund_identity.clone(),
            expiry: self.alpha_expiry,
            secret_hash: self.secret_hash,
        }
    }
}

impl<AL, BL, AA, BA, AI, BI> OngoingSwap<AL, BL, AA, BA, AI, BI>
where
    BL: Clone,
    BA: Clone,
    BI: Clone,
{
    pub fn beta_htlc_params(&self) -> HtlcParams<BL, BA, BI> {
        HtlcParams {
            asset: self.beta_asset.clone(),
            ledger: self.beta_ledger.clone(),
            redeem_identity: self.beta_ledger_redeem_identity.clone(),
            refund_identity: self.beta_ledger_refund_identity.clone(),
            expiry: self.beta_expiry,
            secret_hash: self.secret_hash,
        }
    }
}

pub type SwapEventOnLedger<A> = SwapEvent<
    <A as ActorState>::AA,
    <A as ActorState>::BA,
    <A as ActorState>::AH,
    <A as ActorState>::BH,
    <A as ActorState>::AT,
    <A as ActorState>::BT,
>;

#[derive(Debug, Clone, PartialEq, strum_macros::Display)]
pub enum SwapEvent<AA, BA, AH, BH, AT, BT> {
    AlphaDeployed(Deployed<AH, AT>),
    AlphaFunded(Funded<AA, AT>),
    AlphaRedeemed(Redeemed<AT>),
    AlphaRefunded(Refunded<AT>),

    BetaDeployed(Deployed<BH, BT>),
    BetaFunded(Funded<BA, BT>),
    BetaRedeemed(Redeemed<BT>),
    BetaRefunded(Refunded<BT>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{asset, htlc_location, transaction};

    #[test]
    fn swap_event_should_render_to_nice_string() {
        let event = SwapEvent::<
            asset::Bitcoin,
            asset::Ether,
            htlc_location::Bitcoin,
            htlc_location::Ethereum,
            transaction::Bitcoin,
            transaction::Ethereum,
        >::BetaDeployed(Deployed {
            location: htlc_location::Ethereum::default(),
            transaction: transaction::Ethereum::default(),
        });

        let formatted = format!("{}", event);

        assert_eq!(formatted, "BetaDeployed")
    }
}
