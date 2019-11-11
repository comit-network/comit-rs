use crate::swap_protocols::{
    rfc003::{
        ledger_state::LedgerState,
        state_machine::{
            AlphaDeployed, AlphaFunded, AlphaFundedBetaDeployed, AlphaFundedBetaRedeemed,
            AlphaFundedBetaRefunded, AlphaIncorrectlyFunded, AlphaRedeemedBetaFunded,
            AlphaRefundedBetaFunded, BothFunded, Error as ErrorState, Final, SwapOutcome,
            SwapStates,
        },
        ActorState,
    },
    swap_id::SwapId,
};
use either::Either;
use std::{any::Any, collections::HashMap, sync::Mutex};

#[derive(Debug)]
pub enum Error {
    InvalidType,
}

pub trait StateStore: Send + Sync + 'static {
    fn insert<A: ActorState>(&self, key: SwapId, value: A);
    fn get<A: ActorState>(&self, key: &SwapId) -> Result<Option<A>, Error>;
    fn update<A: ActorState>(&self, key: &SwapId, update: SwapStates<A::AL, A::BL, A::AA, A::BA>);
}

#[derive(Default, Debug)]
pub struct InMemoryStateStore {
    states: Mutex<HashMap<SwapId, Box<dyn Any + Send + Sync>>>,
}

impl StateStore for InMemoryStateStore {
    fn insert<A: ActorState>(&self, key: SwapId, value: A) {
        let mut states = self.states.lock().unwrap();
        states.insert(key, Box::new(value));
    }

    fn get<A: ActorState>(&self, key: &SwapId) -> Result<Option<A>, Error> {
        let states = self.states.lock().unwrap();
        match states.get(key) {
            Some(state) => match state.downcast_ref::<A>() {
                Some(state) => Ok(Some(state.clone())),
                None => Err(Error::InvalidType),
            },
            None => Ok(None),
        }
    }

    fn update<A: ActorState>(&self, key: &SwapId, update: SwapStates<A::AL, A::BL, A::AA, A::BA>) {
        use self::{LedgerState::*, SwapStates as SS};

        let mut actor_state = match self.get::<A>(key) {
            Ok(Some(actor_state)) => actor_state,
            Ok(None) => {
                log::warn!("Value not found for key {}", key);
                return;
            }
            Err(_invalid_type) => {
                log::warn!("Attempted to get state with wrong type for key {}", key);
                return;
            }
        };

        match update {
            SS::Start(_) => {
                log::warn!("Attempted to update Start state for key {}", key);
                return;
            }
            SS::AlphaDeployed(AlphaDeployed { alpha_deployed, .. }) => {
                *actor_state.alpha_ledger_mut() = Deployed {
                    htlc_location: alpha_deployed.location,
                    deploy_transaction: alpha_deployed.transaction,
                }
            }

            SS::AlphaIncorrectlyFunded(AlphaIncorrectlyFunded {
                alpha_deployed,
                alpha_funded,
                ..
            }) => {
                *actor_state.alpha_ledger_mut() = IncorrectlyFunded {
                    htlc_location: alpha_deployed.location,
                    deploy_transaction: alpha_deployed.transaction,
                    fund_transaction: alpha_funded.transaction,
                }
            }
            SS::AlphaFunded(AlphaFunded {
                alpha_deployed,
                alpha_funded,
                ..
            }) => {
                *actor_state.alpha_ledger_mut() = Funded {
                    htlc_location: alpha_deployed.location,
                    deploy_transaction: alpha_deployed.transaction,
                    fund_transaction: alpha_funded.transaction,
                }
            }
            SS::AlphaFundedBetaDeployed(AlphaFundedBetaDeployed {
                alpha_deployed,
                alpha_funded,
                beta_deployed,
                ..
            }) => {
                *actor_state.alpha_ledger_mut() = Funded {
                    htlc_location: alpha_deployed.location,
                    deploy_transaction: alpha_deployed.transaction,
                    fund_transaction: alpha_funded.transaction,
                };
                *actor_state.beta_ledger_mut() = Deployed {
                    htlc_location: beta_deployed.location,
                    deploy_transaction: beta_deployed.transaction,
                };
            }
            SS::BothFunded(BothFunded {
                alpha_deployed,
                alpha_funded,
                beta_deployed,
                beta_funded,
                ..
            }) => {
                *actor_state.alpha_ledger_mut() = Funded {
                    htlc_location: alpha_deployed.location,
                    deploy_transaction: alpha_deployed.transaction,
                    fund_transaction: alpha_funded.transaction,
                };
                *actor_state.beta_ledger_mut() = Funded {
                    htlc_location: beta_deployed.location,
                    deploy_transaction: beta_deployed.transaction,
                    fund_transaction: beta_funded.transaction,
                };
            }
            SS::AlphaFundedBetaRefunded(AlphaFundedBetaRefunded {
                beta_deployed,
                beta_funded,
                beta_refund_transaction,
                ..
            })
            | SS::Final(Final(SwapOutcome::BothRefunded {
                beta_deployed,
                beta_funded,
                alpha_or_beta_refunded: Either::Right(beta_refund_transaction),
                ..
            }))
            | SS::Final(Final(SwapOutcome::AlphaRedeemedBetaRefunded {
                beta_deployed,
                beta_funded,
                alpha_redeemed_or_beta_refunded: Either::Right(beta_refund_transaction),
                ..
            })) => {
                *actor_state.beta_ledger_mut() = Refunded {
                    htlc_location: beta_deployed.location,
                    deploy_transaction: beta_deployed.transaction,
                    fund_transaction: beta_funded.transaction,
                    refund_transaction: beta_refund_transaction.transaction,
                }
            }
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                alpha_deployed,
                alpha_funded,
                alpha_refunded,
                ..
            })
            | SS::Final(Final(SwapOutcome::AlphaRefunded {
                alpha_deployed,
                alpha_funded,
                alpha_refunded,
                ..
            }))
            | SS::Final(Final(SwapOutcome::BothRefunded {
                alpha_deployed,
                alpha_funded,
                alpha_or_beta_refunded: Either::Left(alpha_refunded),
                ..
            }))
            | SS::Final(Final(SwapOutcome::AlphaRefundedBetaRedeemed {
                alpha_deployed,
                alpha_funded,
                alpha_refunded_or_beta_redeemed: Either::Left(alpha_refunded),
                ..
            })) => {
                *actor_state.alpha_ledger_mut() = Refunded {
                    htlc_location: alpha_deployed.location,
                    deploy_transaction: alpha_deployed.transaction,
                    fund_transaction: alpha_funded.transaction,
                    refund_transaction: alpha_refunded.transaction,
                }
            }
            SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                beta_deployed,
                beta_funded,
                beta_redeem_transaction,
                ..
            })
            | SS::Final(Final(SwapOutcome::BothRedeemed {
                beta_deployed,
                beta_funded,
                alpha_or_beta_redeemed: Either::Right(beta_redeem_transaction),
                ..
            }))
            | SS::Final(Final(SwapOutcome::AlphaRefundedBetaRedeemed {
                beta_deployed,
                beta_funded,
                alpha_refunded_or_beta_redeemed: Either::Right(beta_redeem_transaction),
                ..
            })) => {
                *actor_state.beta_ledger_mut() = Redeemed {
                    htlc_location: beta_deployed.location,
                    deploy_transaction: beta_deployed.transaction,
                    fund_transaction: beta_funded.transaction,
                    redeem_transaction: beta_redeem_transaction.transaction,
                };
                actor_state.set_secret(beta_redeem_transaction.secret);
            }
            SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                alpha_deployed,
                alpha_funded,
                alpha_redeemed,
                ..
            })
            | SS::Final(Final(SwapOutcome::AlphaRedeemed {
                alpha_deployed,
                alpha_funded,
                alpha_redeemed,
                ..
            }))
            | SS::Final(Final(SwapOutcome::BothRedeemed {
                alpha_deployed,
                alpha_funded,
                alpha_or_beta_redeemed: Either::Left(alpha_redeemed),
                ..
            }))
            | SS::Final(Final(SwapOutcome::AlphaRedeemedBetaRefunded {
                alpha_deployed,
                alpha_funded,
                alpha_redeemed_or_beta_refunded: Either::Left(alpha_redeemed),
                ..
            })) => {
                *actor_state.alpha_ledger_mut() = Redeemed {
                    htlc_location: alpha_deployed.location,
                    deploy_transaction: alpha_deployed.transaction,
                    fund_transaction: alpha_funded.transaction,
                    redeem_transaction: alpha_redeemed.transaction,
                };
                actor_state.set_secret(alpha_redeemed.secret);
            }
            SS::Error(ErrorState(e)) => {
                log::error!("Internal failure: {:?}", e);
                return;
            }
        }

        self.insert(key.clone(), actor_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        seed::Seed,
        swap_protocols::{
            ledger::{Bitcoin, Ethereum},
            rfc003::{alice, messages::Request, Accept, Secret},
            HashFunction, Timestamp,
        },
    };
    use bitcoin::Amount;
    use ethereum_support::{Address, EtherQuantity};
    use spectral::prelude::*;

    #[test]
    fn insert_and_get_state() {
        let state_store = InMemoryStateStore::default();

        let bitcoin_pub_key = crate::bitcoin::PublicKey::new(
            "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275"
                .parse()
                .unwrap(),
        );
        let ethereum_address: Address = "8457037fcd80a8650c4692d7fcfc1d0a96b92867".parse().unwrap();

        let request = Request {
            id: SwapId::default(),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_asset: Amount::from_btc(1.0).unwrap(),
            beta_asset: EtherQuantity::from_eth(10.0),
            hash_function: HashFunction::Sha256,
            alpha_ledger_refund_identity: bitcoin_pub_key,
            beta_ledger_redeem_identity: ethereum_address,
            alpha_expiry: Timestamp::from(2_000_000_000),
            beta_expiry: Timestamp::from(2_000_000_000),
            secret_hash: Secret::from(*b"hello world, you are beautiful!!").hash(),
        };
        let accept = Accept {
            id: SwapId::default(),
            beta_ledger_refund_identity: ethereum_address,
            alpha_ledger_redeem_identity: bitcoin_pub_key,
        };

        let id = SwapId::default();
        let seed = Seed::from(*b"hello world, you are beautiful!!");
        let secret_source = seed.swap_seed(id);
        let state = alice::State::accepted(request, accept, secret_source);

        state_store
            .insert::<alice::State<Bitcoin, Ethereum, Amount, EtherQuantity>>(id, state.clone());

        let res = state_store
            .get::<alice::State<Bitcoin, Ethereum, Amount, EtherQuantity>>(&id)
            .unwrap();
        assert_that(&res).contains_value(state);
    }
}
