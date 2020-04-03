use crate::{
    swap_protocols::{
        rfc003::{
            self,
            events::{
                Deployed, Funded, HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded, Redeemed,
                Refunded,
            },
            Accept, LedgerState, Request, SecretHash,
        },
        state, HashFunction, InsertFailedSwap, SwapId,
    },
    timestamp::Timestamp,
};
use chrono::NaiveDateTime;
use futures::future::{self, Either};
use genawaiter::{
    sync::{Co, Gen},
    GeneratorState,
};
use std::sync::Arc;

/// Returns a future that tracks the swap negotiated from the given request and
/// accept response on a ledger.
///
/// The current implementation is naive in the sense that it does not take into
/// account situations where it is clear that no more events will happen even
/// though in theory, there could. For example:
/// - funded
/// - refunded
///
/// It is highly unlikely for Bob to fund the HTLC now, yet the current
/// implementation is still waiting for that.
pub async fn create_watcher<D, S, L, A, H, I, T>(
    dependencies: D,
    ledger_state: Arc<S>,
    id: SwapId,
    htlc_params: HtlcParams<L, A, I>,
    accepted_at: NaiveDateTime,
) where
    D: InsertFailedSwap
        + HtlcFunded<L, A, H, I, T>
        + HtlcDeployed<L, A, H, I, T>
        + HtlcRedeemed<L, A, H, I, T>
        + HtlcRefunded<L, A, H, I, T>,
    S: state::Update<SwapEvent<A, H, T>> + state::Insert<LedgerState<A, H, T>>,
    L: Clone,
    A: Ord + Clone,
    H: Clone,
    I: Clone,
    T: Clone,
{
    ledger_state
        .insert(id, LedgerState::<A, H, T>::NotDeployed)
        .await;

    // construct a generator that watches alpha and beta ledger concurrently
    let mut generator = Gen::new({
        |co| async {
            watch_ledger::<D, L, A, H, I, T>(&dependencies, co, htlc_params, accepted_at).await
        }
    });

    loop {
        // wait for events to be emitted as the generator executes
        match generator.async_resume().await {
            // every event that is yielded is passed on
            GeneratorState::Yielded(event) => {
                tracing::info!("swap {} yielded event {}", id, event);
                ledger_state.update(&id, event).await;
            }
            // the generator stopped executing, this means there are no more events that can be
            // watched.
            GeneratorState::Complete(Ok(_)) => {
                tracing::info!("swap {} finished", id);
                return;
            }
            GeneratorState::Complete(Err(e)) => {
                tracing::error!("swap {} failed with {:?}", id, e);
                dependencies.insert_failed_swap(&id).await;
                return;
            }
        }
    }
}

/// Returns a future that waits for events to happen on a ledger.
///
/// Each event is yielded through the controller handle (co) of the coroutine.
async fn watch_ledger<D, L, A, H, I, T>(
    dependencies: &D,
    co: Co<SwapEvent<A, H, T>>,
    htlc_params: HtlcParams<L, A, I>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    D: HtlcFunded<L, A, H, I, T>
        + HtlcDeployed<L, A, H, I, T>
        + HtlcRedeemed<L, A, H, I, T>
        + HtlcRefunded<L, A, H, I, T>,
    Deployed<H, T>: Clone,
    Redeemed<T>: Clone,
    Refunded<T>: Clone,
{
    let deployed = dependencies
        .htlc_deployed(&htlc_params, start_of_swap)
        .await?;
    co.yield_(SwapEvent::Deployed(deployed.clone())).await;

    let funded = dependencies
        .htlc_funded(&htlc_params, &deployed, start_of_swap)
        .await?;
    co.yield_(SwapEvent::Funded(funded)).await;

    let redeemed = dependencies.htlc_redeemed(&htlc_params, &deployed, start_of_swap);

    let refunded = dependencies.htlc_refunded(&htlc_params, &deployed, start_of_swap);

    match future::try_select(redeemed, refunded).await {
        Ok(Either::Left((redeemed, _))) => {
            co.yield_(SwapEvent::Redeemed(redeemed.clone())).await;
        }
        Ok(Either::Right((refunded, _))) => {
            co.yield_(SwapEvent::Refunded(refunded.clone())).await;
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

#[derive(Debug, Clone, PartialEq, strum_macros::Display)]
pub enum SwapEvent<A, H, T> {
    Deployed(Deployed<H, T>),
    Funded(Funded<A, T>),
    Redeemed(Redeemed<T>),
    Refunded(Refunded<T>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{asset, htlc_location, transaction};

    #[test]
    fn swap_event_should_render_to_nice_string() {
        let event =
            SwapEvent::<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>::Deployed(
                Deployed {
                    location: htlc_location::Ethereum::default(),
                    transaction: transaction::Ethereum::default(),
                },
            );

        let formatted = format!("{}", event);

        assert_eq!(formatted, "Deployed")
    }
}
