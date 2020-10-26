use crate::{
    btsieve::{BlockByHash, LatestBlock},
    ledger, state,
    state::Update,
    storage::Storage,
    LocalSwapId, Role, Side,
};
use anyhow::Result;
use bitcoin::{Block, BlockHash};
use comit::{asset, htlc_location, Secret};
use futures::TryStreamExt;
use std::collections::{hash_map::Entry, HashMap};
use time::OffsetDateTime;
use tokio::sync::Mutex;

use crate::btsieve::ConnectedNetwork;
pub use comit::{hbit::*, identity};

/// Creates a new instance of the hbit protocol, annotated with tracing spans
/// and saves all events in the `States` hashmap.
///
/// This wrapper functions allows us to reuse code within `cnd` without having
/// to give knowledge about tracing or the state hashmaps to the `comit` crate.
#[tracing::instrument(name = "hbit", level = "error", skip(params, start_of_swap, storage, connector), fields(%id, %role, %side))]
pub async fn new<C>(
    id: LocalSwapId,
    params: Params,
    start_of_swap: OffsetDateTime,
    role: Role,
    side: Side,
    storage: Storage,
    connector: impl AsRef<C>,
) -> Result<()>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = BlockHash>
        + ConnectedNetwork<Network = ledger::Bitcoin>,
{
    let mut events = comit::hbit::new(connector.as_ref(), params, start_of_swap);

    while let Some(event) = events.try_next().await? {
        tracing::info!("yielded event {}", event);
        storage.hbit_states.update(&id, event).await;
    }

    tracing::info!("finished");

    Ok(())
}

#[derive(Default, Debug)]
pub struct States(Mutex<HashMap<LocalSwapId, State>>);

impl State {
    pub fn transition_to_funded(&mut self, funded: Funded) {
        match std::mem::replace(self, State::None) {
            State::None => match funded {
                Funded::Correctly { asset, location } => {
                    *self = State::Funded {
                        htlc_location: location,
                        asset,
                    }
                }
                Funded::Incorrectly { asset, location } => {
                    *self = State::IncorrectlyFunded {
                        htlc_location: location,
                        asset,
                    }
                }
            },
            other => panic!("expected state None, got {}", other),
        }
    }

    pub fn transition_to_redeemed(&mut self, redeemed: Redeemed) {
        let Redeemed {
            transaction,
            secret,
        } = redeemed;

        match std::mem::replace(self, State::None) {
            State::Funded {
                htlc_location,
                asset,
            } => {
                *self = State::Redeemed {
                    htlc_location,
                    redeem_transaction: transaction,
                    asset,
                    secret,
                }
            }
            other => panic!("expected state Funded, got {}", other),
        }
    }

    pub fn transition_to_refunded(&mut self, refunded: Refunded) {
        let Refunded { transaction } = refunded;

        match std::mem::replace(self, State::None) {
            State::Funded {
                htlc_location,
                asset,
            }
            | State::IncorrectlyFunded {
                htlc_location,
                asset,
            } => {
                *self = State::Refunded {
                    htlc_location,
                    refund_transaction: transaction,
                    asset,
                }
            }
            other => panic!("expected state Funded or IncorrectlyFunded, got {}", other),
        }
    }
}

#[async_trait::async_trait]
impl state::Get<State> for States {
    async fn get(&self, key: &LocalSwapId) -> anyhow::Result<Option<State>> {
        let states = self.0.lock().await;
        let state = states.get(key).cloned();

        Ok(state)
    }
}

#[async_trait::async_trait]
impl state::Update<Event> for States {
    async fn update(&self, key: &LocalSwapId, event: Event) {
        let mut states = self.0.lock().await;
        let entry = states.entry(*key);

        match (event, entry) {
            (Event::Started, Entry::Vacant(vacant)) => {
                vacant.insert(State::None);
            }
            (Event::Funded(funded), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_funded(funded)
            }
            (Event::Redeemed(redeemed), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_redeemed(redeemed)
            }
            (Event::Refunded(refunded), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_refunded(refunded)
            }
            (Event::Started, Entry::Occupied(_)) => {
                tracing::warn!(
                    "Received Started event for {} although state is already present",
                    key
                );
            }
            (_, Entry::Vacant(_)) => {
                tracing::warn!("State not found for {}", key);
            }
        }
    }
}

/// Represents states that an Bitcoin HTLC can be in.
#[derive(Debug, Clone, Copy, strum_macros::Display)]
pub enum State {
    None,
    Funded {
        htlc_location: htlc_location::Bitcoin,
        asset: asset::Bitcoin,
    },
    IncorrectlyFunded {
        htlc_location: htlc_location::Bitcoin,
        asset: asset::Bitcoin,
    },
    Redeemed {
        htlc_location: htlc_location::Bitcoin,
        redeem_transaction: bitcoin::Txid,
        asset: asset::Bitcoin,
        secret: Secret,
    },
    Refunded {
        htlc_location: htlc_location::Bitcoin,
        refund_transaction: bitcoin::Txid,
        asset: asset::Bitcoin,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Identities {
    pub redeem_identity: identity::Bitcoin,
    pub refund_identity: identity::Bitcoin,
}
