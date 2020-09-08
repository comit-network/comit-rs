use crate::{
    btsieve::{BlockByHash, LatestBlock},
    ledger, state,
    state::Update,
    tracing_ext::InstrumentProtocol,
    LocalSwapId, Role, Side,
};
use bitcoin::{Address, Block, BlockHash};
use chrono::{DateTime, Utc};
use comit::{asset, htlc_location, transaction, LockProtocol, Secret};
pub use comit::{hbit::*, identity};
use futures::TryStreamExt;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};
use tokio::sync::Mutex;

/// Creates a new instance of the hbit protocol, annotated with tracing spans
/// and saves all events in the `States` hashmap.
///
/// This wrapper functions allows us to reuse code within `cnd` without having
/// to give knowledge about tracing or the state hashmaps to the `comit` crate.
pub async fn new<C>(
    id: LocalSwapId,
    params: Params,
    start_of_swap: DateTime<Utc>,
    role: Role,
    side: Side,
    states: Arc<States>,
    connector: impl AsRef<C>,
) where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = BlockHash>,
{
    let mut events = comit::hbit::new(connector.as_ref(), params, start_of_swap)
        .instrument_protocol(id, role, side, LockProtocol::Hbit)
        .inspect_ok(|event| tracing::info!("yielded event {}", event))
        .inspect_err(|error| tracing::error!("swap failed with {:?}", error));

    while let Ok(Some(event)) = events.try_next().await {
        states.update(&id, event).await;
    }

    tracing::info!("swap finished");
}

/// Data required to create a swap that involves Bitcoin.
#[derive(Clone, Debug)]
pub struct CreatedSwap {
    pub amount: asset::Bitcoin,
    pub final_identity: Address,
    pub network: ledger::Bitcoin,
    pub absolute_expiry: u32,
}

#[derive(Default, Debug)]
pub struct States(Mutex<HashMap<LocalSwapId, State>>);

impl State {
    pub fn transition_to_funded(&mut self, funded: Funded) {
        match std::mem::replace(self, State::None) {
            State::None => match funded {
                Funded::Correctly {
                    asset,
                    transaction,
                    location,
                } => {
                    *self = State::Funded {
                        htlc_location: location,
                        fund_transaction: transaction,
                        asset,
                    }
                }
                Funded::Incorrectly {
                    asset,
                    transaction,
                    location,
                } => {
                    *self = State::IncorrectlyFunded {
                        htlc_location: location,
                        fund_transaction: transaction,
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
                fund_transaction,
            } => {
                *self = State::Redeemed {
                    htlc_location,
                    fund_transaction,
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
                fund_transaction,
            }
            | State::IncorrectlyFunded {
                htlc_location,
                asset,
                fund_transaction,
            } => {
                *self = State::Refunded {
                    htlc_location,
                    fund_transaction,
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
#[derive(Debug, Clone, strum_macros::Display)]
pub enum State {
    None,
    Funded {
        htlc_location: htlc_location::Bitcoin,
        fund_transaction: transaction::Bitcoin,
        asset: asset::Bitcoin,
    },
    IncorrectlyFunded {
        htlc_location: htlc_location::Bitcoin,
        fund_transaction: transaction::Bitcoin,
        asset: asset::Bitcoin,
    },
    Redeemed {
        htlc_location: htlc_location::Bitcoin,
        fund_transaction: transaction::Bitcoin,
        redeem_transaction: transaction::Bitcoin,
        asset: asset::Bitcoin,
        secret: Secret,
    },
    Refunded {
        htlc_location: htlc_location::Bitcoin,
        fund_transaction: transaction::Bitcoin,
        refund_transaction: transaction::Bitcoin,
        asset: asset::Bitcoin,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Identities {
    pub redeem_identity: identity::Bitcoin,
    pub refund_identity: identity::Bitcoin,
}
