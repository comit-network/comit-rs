use crate::{
    asset, htlc_location, identity,
    swap_protocols::{state, Ledger, LocalSwapId},
    timestamp::Timestamp,
    transaction,
};
use chrono::{NaiveDateTime, Utc};
use futures::future::{self, Either};
use genawaiter::sync::{Co, Gen};

use crate::{
    btsieve::ethereum::{Cache, Web3Connector},
    swap_protocols::{
        ledger,
        rfc003::{
            create_swap::{HtlcParams, SwapEvent},
            events::{
                Deployed, HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded, Redeemed, Refunded,
            },
            LedgerState,
        },
        LedgerStates, Role,
    },
};
use genawaiter::GeneratorState;
use std::sync::Arc;
use tracing_futures::Instrument;

// Temporary file for spinning up the ledger watcher for Erc20-Halight swaps.

/// Data required to create a swap that involves an ERC20 token.
#[derive(Clone, Debug, PartialEq)]
pub struct CreatedSwap {
    pub amount: asset::Erc20Quantity,
    pub identity: identity::Ethereum,
    pub chain_id: u32,
    pub token_contract: identity::Ethereum,
    pub absolute_expiry: u32,
}

/// Herc20 specific data for an in progress swap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InProgressSwap {
    pub ledger: Ledger,
    pub refund_identity: identity::Ethereum,
    pub redeem_identity: identity::Ethereum,
    pub expiry: Timestamp, // This is the absolute_expiry for now.
}

pub async fn new_herc20_swap(
    swap_id: LocalSwapId,
    connector: Arc<Cache<Web3Connector>>,
    ethereum_ledger_state: Arc<LedgerStates>,
    htlc_params: HtlcParams<ledger::Ethereum, asset::Erc20, identity::Ethereum>,
    role: Role,
) {
    create_watcher::<_, _, _, _, htlc_location::Ethereum, _, transaction::Ethereum>(
        connector.as_ref(),
        ethereum_ledger_state,
        swap_id,
        htlc_params,
        Utc::now().naive_local(),
    )
    .instrument(tracing::error_span!("alpha_ledger", swap_id = %swap_id, role = %role))
    .await
}

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
async fn create_watcher<C, S, L, A, H, I, T>(
    ethereum_connector: &C,
    ledger_state: Arc<S>,
    swap_id: LocalSwapId,
    htlc_params: HtlcParams<L, A, I>,
    accepted_at: NaiveDateTime,
) where
    C: HtlcFunded<L, A, H, I, T>
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
        .insert(swap_id, LedgerState::<A, H, T>::NotDeployed)
        .await;

    // construct a generator that watches alpha and beta ledger concurrently
    let mut generator = Gen::new({
        |co| async {
            watch_ledger::<C, L, A, H, I, T>(&ethereum_connector, co, htlc_params, accepted_at)
                .await
        }
    });

    loop {
        // wait for events to be emitted as the generator executes
        match generator.async_resume().await {
            // every event that is yielded is passed on
            GeneratorState::Yielded(event) => {
                tracing::info!("swap {} yielded event {}", swap_id, event);
                ledger_state.update(&swap_id, event).await;
            }
            // the generator stopped executing, this means there are no more events that can be
            // watched.
            GeneratorState::Complete(Ok(_)) => {
                tracing::info!("swap {} finished", swap_id);
                return;
            }
            GeneratorState::Complete(Err(e)) => {
                tracing::error!("swap {} failed with {:?}", swap_id, e);
                return;
            }
        }
    }
}

/// Returns a future that waits for events to happen on a ledger.
///
/// Each event is yielded through the controller handle (co) of the coroutine.
async fn watch_ledger<C, L, A, H, I, T>(
    ethereum_connector: &C,
    co: Co<SwapEvent<A, H, T>>,
    htlc_params: HtlcParams<L, A, I>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    C: HtlcFunded<L, A, H, I, T>
        + HtlcDeployed<L, A, H, I, T>
        + HtlcRedeemed<L, A, H, I, T>
        + HtlcRefunded<L, A, H, I, T>,
    Deployed<H, T>: Clone,
    Redeemed<T>: Clone,
    Refunded<T>: Clone,
{
    let deployed = ethereum_connector
        .htlc_deployed(&htlc_params, start_of_swap)
        .await?;
    co.yield_(SwapEvent::Deployed(deployed.clone())).await;

    let funded = ethereum_connector
        .htlc_funded(&htlc_params, &deployed, start_of_swap)
        .await?;
    co.yield_(SwapEvent::Funded(funded)).await;

    let redeemed = ethereum_connector.htlc_redeemed(&htlc_params, &deployed, start_of_swap);

    let refunded = ethereum_connector.htlc_refunded(&htlc_params, &deployed, start_of_swap);

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
