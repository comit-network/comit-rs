pub mod events;
mod extract_secret;
pub mod htlc_events;
mod ledger_state;
mod ledger_states;

pub use self::{extract_secret::*, htlc_events::*, ledger_state::*, ledger_states::*};

use crate::{
    asset,
    btsieve::bitcoin::{BitcoindConnector, Cache},
    identity,
    swap_protocols::{
        hbit::{
            events::{
                Deployed, Funded, HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded, Redeemed,
                Refunded,
            },
            LedgerStates,
        },
        ledger, LocalSwapId, Role, SecretHash,
    },
    timestamp::Timestamp,
};
use ::bitcoin::{
    hashes::{hash160, Hash},
    Address,
};
use blockchain_contracts::bitcoin::rfc003::bitcoin_htlc::BitcoinHtlc;
use chrono::{NaiveDateTime, Utc};
use futures::future::{self, Either};
use genawaiter::{
    sync::{Co, Gen},
    GeneratorState,
};
use std::sync::Arc;
use tracing_futures::Instrument;

/// Htlc Bitcoin atomic swap protocol.

/// Data required to create a swap that involves Bitcoin.
#[derive(Clone, Copy, Debug)]
pub struct CreatedSwap {
    pub amount: asset::Bitcoin,
    pub identity: identity::Bitcoin,
    pub network: ledger::Bitcoin,
    pub absolute_expiry: u32,
}

pub async fn new_hbit_swap(
    swap_id: LocalSwapId,
    connector: Arc<Cache<BitcoindConnector>>,
    ledger_states: Arc<LedgerStates>,
    htlc_params: Params,
    role: Role,
) {
    create_watcher(
        connector,
        ledger_states,
        swap_id,
        htlc_params,
        Utc::now().naive_local(),
    )
    .instrument(tracing::error_span!("hbit", swap_id = %swap_id, role = %role))
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
async fn create_watcher(
    connector: Arc<Cache<BitcoindConnector>>,
    ledger_states: Arc<LedgerStates>,
    swap_id: LocalSwapId,
    htlc_params: Params,
    accepted_at: NaiveDateTime,
) {
    ledger_states
        .insert(swap_id, LedgerState::NotDeployed)
        .await;

    // construct a generator that watches alpha and beta ledger concurrently
    let mut generator =
        Gen::new({ |co| async { watch_ledger(connector, co, htlc_params, accepted_at).await } });

    loop {
        // wait for events to be emitted as the generator executes
        match generator.async_resume().await {
            // every event that is yielded is passed on
            GeneratorState::Yielded(event) => {
                tracing::info!("swap {} yielded event {}", swap_id, event);
                ledger_states.update(&swap_id, event).await;
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
async fn watch_ledger(
    connector: Arc<Cache<BitcoindConnector>>,
    co: Co<Event>,
    htlc_params: Params,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()> {
    let deployed = connector.htlc_deployed(&htlc_params, start_of_swap).await?;
    co.yield_(Event::Deployed(deployed.clone())).await;

    let funded = connector
        .htlc_funded(&htlc_params, &deployed, start_of_swap)
        .await?;
    co.yield_(Event::Funded(funded)).await;

    let redeemed = connector.htlc_redeemed(&htlc_params, &deployed, start_of_swap);

    let refunded = connector.htlc_refunded(&htlc_params, &deployed, start_of_swap);

    match future::try_select(redeemed, refunded).await {
        Ok(Either::Left((redeemed, _))) => {
            co.yield_(Event::Redeemed(redeemed.clone())).await;
        }
        Ok(Either::Right((refunded, _))) => {
            co.yield_(Event::Refunded(refunded.clone())).await;
        }
        Err(either) => {
            let (error, _other_future) = either.factor_first();

            return Err(error);
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub network: bitcoin::Network,
    pub asset: asset::Bitcoin,
    pub redeem_identity: identity::Bitcoin,
    pub refund_identity: identity::Bitcoin,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl From<Params> for BitcoinHtlc {
    fn from(htlc_params: Params) -> Self {
        let refund_public_key = ::bitcoin::PublicKey::from(htlc_params.refund_identity);
        let redeem_public_key = ::bitcoin::PublicKey::from(htlc_params.redeem_identity);

        let refund_identity = hash160::Hash::hash(&refund_public_key.key.serialize());
        let redeem_identity = hash160::Hash::hash(&redeem_public_key.key.serialize());

        BitcoinHtlc::new(
            htlc_params.expiry.into(),
            refund_identity,
            redeem_identity,
            htlc_params.secret_hash.into_raw(),
        )
    }
}

impl Params {
    pub fn compute_address(&self) -> Address {
        BitcoinHtlc::from(*self).compute_address(self.network)
    }
}

#[derive(Debug, Clone, PartialEq, strum_macros::Display)]
pub enum Event {
    Deployed(Deployed),
    Funded(Funded),
    Redeemed(Redeemed),
    Refunded(Refunded),
}
