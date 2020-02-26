use crate::{
    asset::Asset,
    db::AcceptedSwap,
    swap_protocols::{
        rfc003::{
            self,
            events::{
                Deployed, Funded, HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded, Redeemed,
                Refunded,
            },
            ledger::Ledger,
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
/// accept response on both ledgers.
///
/// The current implementation is naive in the sense that it does not take into
/// account situations where it is clear that no more events will happen even
/// though in theory, there could. For example:
/// - alpha funded
/// - alpha refunded
///
/// It is highly unlikely for Bob to fund the HTLC now, yet the current
/// implementation is still waiting for that.
pub async fn create_swap<D, A: ActorState>(
    dependencies: D,
    accepted: AcceptedSwap<A::AL, A::BL, A::AA, A::BA>,
) where
    D: StateStore
        + HtlcFunded<A::AL, A::AA>
        + HtlcFunded<A::BL, A::BA>
        + HtlcDeployed<A::AL, A::AA>
        + HtlcDeployed<A::BL, A::BA>
        + HtlcRedeemed<A::AL, A::AA>
        + HtlcRedeemed<A::BL, A::BA>
        + HtlcRefunded<A::AL, A::AA>
        + HtlcRefunded<A::BL, A::BA>
        + Clone,
{
    let (request, accept, at) = accepted.clone();

    let id = request.swap_id;
    let swap = OngoingSwap::new(request, accept);

    // construct a generator that watches alpha and beta ledger concurrently
    let mut generator = Gen::new({
        let dependencies = dependencies.clone();
        |co| async move {
            future::try_join(
                watch_alpha_ledger::<_, A::AL, _, A::BL, _>(
                    &dependencies,
                    &co,
                    swap.alpha_htlc_params(),
                    at,
                ),
                watch_beta_ledger::<_, A::AL, _, A::BL, _>(
                    &dependencies,
                    &co,
                    swap.beta_htlc_params(),
                    at,
                ),
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
async fn watch_alpha_ledger<D, AL, AA, BL, BA>(
    dependencies: &D,
    co: &Co<SwapEventOnLedger<AL, BL, AA, BA>>,
    htlc_params: HtlcParams<AL, AA, AL::Identity>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
    D: HtlcFunded<AL, AA> + HtlcDeployed<AL, AA> + HtlcRedeemed<AL, AA> + HtlcRefunded<AL, AA>,
{
    let deployed = dependencies
        .htlc_deployed(htlc_params.clone(), start_of_swap)
        .await?;
    co.yield_(SwapEvent::AlphaDeployed(deployed.clone())).await;

    let funded = dependencies
        .htlc_funded(htlc_params.clone(), &deployed, start_of_swap)
        .await?;
    co.yield_(SwapEvent::AlphaFunded(funded.clone())).await;

    let redeemed = dependencies.htlc_redeemed(htlc_params.clone(), &deployed, start_of_swap);

    let refunded = dependencies.htlc_refunded(htlc_params, &deployed, start_of_swap);

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
async fn watch_beta_ledger<D, AL, AA, BL, BA>(
    dependencies: &D,
    co: &Co<SwapEventOnLedger<AL, BL, AA, BA>>,
    htlc_params: HtlcParams<BL, BA, BL::Identity>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
    D: HtlcFunded<BL, BA> + HtlcDeployed<BL, BA> + HtlcRedeemed<BL, BA> + HtlcRefunded<BL, BA>,
{
    let deployed = dependencies
        .htlc_deployed(htlc_params.clone(), start_of_swap)
        .await?;
    co.yield_(SwapEvent::BetaDeployed(deployed.clone())).await;

    let funded = dependencies
        .htlc_funded(htlc_params.clone(), &deployed, start_of_swap)
        .await?;
    co.yield_(SwapEvent::BetaFunded(funded.clone())).await;

    let redeemed = dependencies.htlc_redeemed(htlc_params.clone(), &deployed, start_of_swap);

    let refunded = dependencies.htlc_refunded(htlc_params, &deployed, start_of_swap);

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
pub struct HtlcParams<L, A, I>
where
    A: Asset,
{
    pub asset: A,
    pub ledger: L,
    pub redeem_identity: I,
    pub refund_identity: I,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl<L, A> HtlcParams<L, A, L::Identity>
where
    L: Ledger,
    A: Asset,
{
    pub fn new_alpha_params<BL, BA>(
        request: &rfc003::Request<L, BL, A, BA>,
        accept_response: &rfc003::Accept<L::Identity, BL::Identity>,
    ) -> Self
    where
        BL: Ledger,
        BA: Asset,
    {
        HtlcParams {
            asset: request.alpha_asset.clone(),
            ledger: request.alpha_ledger,
            redeem_identity: accept_response.alpha_ledger_redeem_identity,
            refund_identity: request.alpha_ledger_refund_identity,
            expiry: request.alpha_expiry,
            secret_hash: request.secret_hash,
        }
    }

    pub fn new_beta_params<AL, AA>(
        request: &rfc003::Request<AL, L, AA, A>,
        accept_response: &rfc003::Accept<AL::Identity, L::Identity>,
    ) -> Self
    where
        AL: Ledger,
        AA: Asset,
    {
        HtlcParams {
            asset: request.beta_asset.clone(),
            ledger: request.beta_ledger,
            redeem_identity: request.beta_ledger_redeem_identity,
            refund_identity: accept_response.beta_ledger_refund_identity,
            expiry: request.beta_expiry,
            secret_hash: request.secret_hash,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OngoingSwap<AL, BL, AA, BA>
where
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
{
    pub alpha_ledger: AL,
    pub beta_ledger: BL,
    pub alpha_asset: AA,
    pub beta_asset: BA,
    pub hash_function: HashFunction,
    pub alpha_ledger_redeem_identity: AL::Identity,
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_redeem_identity: BL::Identity,
    pub beta_ledger_refund_identity: BL::Identity,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl<AL, BL, AA, BA> OngoingSwap<AL, BL, AA, BA>
where
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
{
    pub fn new(
        request: Request<AL, BL, AA, BA>,
        accept: Accept<AL::Identity, BL::Identity>,
    ) -> Self {
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

    pub fn alpha_htlc_params(&self) -> HtlcParams<AL, AA, AL::Identity> {
        HtlcParams {
            asset: self.alpha_asset.clone(),
            ledger: self.alpha_ledger,
            redeem_identity: self.alpha_ledger_redeem_identity,
            refund_identity: self.alpha_ledger_refund_identity,
            expiry: self.alpha_expiry,
            secret_hash: self.secret_hash,
        }
    }

    pub fn beta_htlc_params(&self) -> HtlcParams<BL, BA, BL::Identity> {
        HtlcParams {
            asset: self.beta_asset.clone(),
            ledger: self.beta_ledger,
            redeem_identity: self.beta_ledger_redeem_identity,
            refund_identity: self.beta_ledger_refund_identity,
            expiry: self.beta_expiry,
            secret_hash: self.secret_hash,
        }
    }
}

pub type SwapEventOnLedger<AL, BL, AA, BA> = SwapEvent<
    <AL as Ledger>::HtlcLocation,
    <AL as Ledger>::Transaction,
    <BL as Ledger>::HtlcLocation,
    <BL as Ledger>::Transaction,
    AA,
    BA,
>;

#[derive(Debug, Clone, PartialEq, strum_macros::Display)]
pub enum SwapEvent<AH, AT, BH, BT, AA, BA>
where
    AA: Asset,
    BA: Asset,
{
    AlphaDeployed(Deployed<AT, AH>),
    AlphaFunded(Funded<AT, AA>),
    AlphaRedeemed(Redeemed<AT>),
    AlphaRefunded(Refunded<AT>),

    BetaDeployed(Deployed<BT, BH>),
    BetaFunded(Funded<BT, BA>),
    BetaRedeemed(Redeemed<BT>),
    BetaRefunded(Refunded<BT>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{asset, ethereum};

    #[test]
    fn swap_event_should_render_to_nice_string() {
        let event = SwapEvent::<
            ::bitcoin::OutPoint,
            ::bitcoin::Transaction,
            crate::ethereum::Address,
            crate::ethereum::Transaction,
            asset::Bitcoin,
            asset::Ether,
        >::BetaDeployed(Deployed {
            transaction: ethereum::Transaction::default(),
            location: ethereum::Address::default(),
        });

        let formatted = format!("{}", event);

        assert_eq!(formatted, "BetaDeployed")
    }
}
