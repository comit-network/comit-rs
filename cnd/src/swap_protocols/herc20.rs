use crate::swap_protocols::{state, Ledger, LocalSwapId};
use comit::{asset, htlc_location, transaction, Secret, SecretHash, Timestamp};
use std::collections::{hash_map::Entry, HashMap};
use tokio::sync::Mutex;

use crate::identity;
use blockchain_contracts::ethereum::rfc003::Erc20Htlc;
pub use comit::herc20::*;

/// Asset of the HErc20 protocol.
///
/// To be used when transferring ERC20 on Ethereum with the HErc20 protocol.
#[derive(Debug, Clone)]
pub struct Asset(pub asset::Erc20);

/// Herc20 specific data for an in progress swap.
#[derive(Debug, Clone, PartialEq)]
pub struct InProgressSwap {
    pub asset: asset::Erc20,
    pub ledger: Ledger,
    pub refund_identity: identity::Ethereum,
    pub redeem_identity: identity::Ethereum,
    pub expiry: Timestamp, // This is the absolute_expiry for now.
}

#[derive(Default, Debug)]
pub struct States(Mutex<HashMap<LocalSwapId, State>>);

impl State {
    pub fn transition_to_deployed(&mut self, deployed: Deployed) {
        let Deployed {
            transaction,
            location,
        } = deployed;

        match std::mem::replace(self, State::None) {
            State::None => {
                *self = State::Deployed {
                    deploy_transaction: transaction,
                    htlc_location: location,
                }
            }
            other => panic!("expected state NotDeployed, got {}", other),
        }
    }

    pub fn transition_to_funded(&mut self, funded: Funded) {
        match std::mem::replace(self, State::None) {
            State::Deployed {
                deploy_transaction,
                htlc_location,
            } => match funded {
                Funded::Correctly { asset, transaction } => {
                    *self = State::Funded {
                        deploy_transaction,
                        htlc_location,
                        fund_transaction: transaction,
                        asset,
                    }
                }
                Funded::Incorrectly { asset, transaction } => {
                    *self = State::IncorrectlyFunded {
                        deploy_transaction,
                        htlc_location,
                        fund_transaction: transaction,
                        asset,
                    }
                }
            },
            other => panic!("expected state Deployed, got {}", other),
        }
    }

    pub fn transition_to_redeemed(&mut self, redeemed: Redeemed) {
        let Redeemed {
            transaction,
            secret,
        } = redeemed;

        match std::mem::replace(self, State::None) {
            State::Funded {
                deploy_transaction,
                htlc_location,
                asset,
                fund_transaction,
            } => {
                *self = State::Redeemed {
                    deploy_transaction,
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
                deploy_transaction,
                htlc_location,
                asset,
                fund_transaction,
            }
            | State::IncorrectlyFunded {
                deploy_transaction,
                htlc_location,
                asset,
                fund_transaction,
            } => {
                *self = State::Refunded {
                    deploy_transaction,
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
            (Event::Deployed(deployed), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_deployed(deployed)
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

/// Represents states that an ERC20 HTLC can be in.
#[derive(Debug, Clone, strum_macros::Display)]
#[allow(clippy::large_enum_variant)]
pub enum State {
    None,
    Deployed {
        htlc_location: htlc_location::Ethereum,
        deploy_transaction: transaction::Ethereum,
    },
    Funded {
        htlc_location: htlc_location::Ethereum,
        deploy_transaction: transaction::Ethereum,
        fund_transaction: transaction::Ethereum,
        asset: asset::Erc20,
    },
    IncorrectlyFunded {
        htlc_location: htlc_location::Ethereum,
        deploy_transaction: transaction::Ethereum,
        fund_transaction: transaction::Ethereum,
        asset: asset::Erc20,
    },
    Redeemed {
        htlc_location: htlc_location::Ethereum,
        deploy_transaction: transaction::Ethereum,
        fund_transaction: transaction::Ethereum,
        redeem_transaction: transaction::Ethereum,
        asset: asset::Erc20,
        secret: Secret,
    },
    Refunded {
        htlc_location: htlc_location::Ethereum,
        deploy_transaction: transaction::Ethereum,
        fund_transaction: transaction::Ethereum,
        refund_transaction: transaction::Ethereum,
        asset: asset::Erc20,
    },
}

impl InProgressSwap {
    pub fn build_erc20_htlc(&self, secret_hash: SecretHash) -> Erc20Htlc {
        let refund_address = blockchain_contracts::ethereum::Address(self.refund_identity.into());
        let redeem_address = blockchain_contracts::ethereum::Address(self.redeem_identity.into());
        let token_contract_address =
            blockchain_contracts::ethereum::Address(self.asset.token_contract.into());

        Erc20Htlc::new(
            self.expiry.into(),
            refund_address,
            redeem_address,
            secret_hash.into(),
            token_contract_address,
            self.asset.quantity.clone().into(),
        )
    }
}
