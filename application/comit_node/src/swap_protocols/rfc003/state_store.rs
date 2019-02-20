use crate::swap_protocols::{
    rfc003::{
        ledger_state::LedgerState,
        messages::AcceptResponseBody,
        state_machine::{
            Accepted, AlphaDeployed, AlphaFunded, AlphaFundedBetaDeployed, AlphaFundedBetaRedeemed,
            AlphaFundedBetaRefunded, AlphaRedeemedBetaFunded, AlphaRefundedBetaFunded, BothFunded,
            Error as ErrorState, Final, SwapOutcome, SwapStates,
        },
        ActorState,
    },
    swap_id::SwapId,
};
use either::Either;
use std::{any::Any, collections::HashMap, hash::Hash, sync::Mutex};

#[derive(Debug)]
pub enum Error {
    InvalidType,
}

pub trait StateStore: Send + Sync + 'static {
    fn insert<A: ActorState>(&self, key: SwapId, value: A);
    fn get<A: ActorState>(&self, key: SwapId) -> Result<Option<A>, Error>;
    fn update<A: ActorState>(&self, key: SwapId, update: SwapStates<A::AL, A::BL, A::AA, A::BA>);
}

#[derive(Default, Debug)]
pub struct InMemoryStateStore<K: Hash + Eq> {
    states: Mutex<HashMap<K, Box<dyn Any + Send + Sync>>>,
}

impl StateStore for InMemoryStateStore<SwapId> {
    fn insert<A: ActorState>(&self, key: SwapId, value: A) {
        let mut states = self.states.lock().unwrap();
        states.insert(key, Box::new(value));
    }

    fn get<A: ActorState>(&self, key: SwapId) -> Result<Option<A>, Error> {
        let states = self.states.lock().unwrap();
        match states.get(&key) {
            Some(state) => match state.downcast_ref::<A>() {
                Some(state) => Ok(Some(state.clone())),
                None => Err(Error::InvalidType),
            },
            None => Ok(None),
        }
    }

    fn update<A: ActorState>(&self, key: SwapId, update: SwapStates<A::AL, A::BL, A::AA, A::BA>) {
        use self::{LedgerState::*, SwapStates as SS};

        let mut actor_state = match self.get::<A>(key) {
            Ok(Some(actor_state)) => actor_state,
            Ok(None) => {
                warn!("Value not found for key {}", key);
                return;
            }
            Err(_invalid_type) => {
                warn!("Attempted to get state with wrong type for key {}", key);
                return;
            }
        };

        match update {
            SS::Start(_) => {
                warn!("Attempted to save Start state for key {}", key);
                return;
            }
            SS::Accepted(Accepted { swap }) => actor_state.set_response(Ok(AcceptResponseBody {
                alpha_ledger_redeem_identity: swap.alpha_ledger_redeem_identity,
                beta_ledger_refund_identity: swap.beta_ledger_refund_identity,
            })),
            SS::Final(Final(SwapOutcome::Rejected { rejection_type, .. })) => {
                actor_state.set_response(Err(rejection_type))
            }
            SS::AlphaDeployed(AlphaDeployed {
                alpha_deploy_transaction,
                ..
            }) => {
                *actor_state.alpha_ledger_mut() = Deployed {
                    htlc_location: alpha_deploy_transaction.location,
                    deploy_transaction: alpha_deploy_transaction.transaction,
                }
            }
            SS::AlphaFunded(AlphaFunded {
                alpha_deploy_transaction,
                alpha_fund_transaction,
                ..
            }) => {
                *actor_state.alpha_ledger_mut() = Funded {
                    htlc_location: alpha_deploy_transaction.location,
                    deploy_transaction: alpha_deploy_transaction.transaction,
                    fund_transaction: alpha_fund_transaction.transaction,
                }
            }
            SS::AlphaFundedBetaDeployed(AlphaFundedBetaDeployed {
                alpha_deploy_transaction,
                alpha_fund_transaction,
                beta_deploy_transaction,
                ..
            }) => {
                *actor_state.alpha_ledger_mut() = Funded {
                    htlc_location: alpha_deploy_transaction.location,
                    deploy_transaction: alpha_deploy_transaction.transaction,
                    fund_transaction: alpha_fund_transaction.transaction,
                };
                *actor_state.beta_ledger_mut() = Deployed {
                    htlc_location: beta_deploy_transaction.location,
                    deploy_transaction: beta_deploy_transaction.transaction,
                };
            }
            SS::BothFunded(BothFunded {
                alpha_deploy_transaction,
                alpha_fund_transaction,
                beta_deploy_transaction,
                beta_fund_transaction,
                ..
            }) => {
                *actor_state.alpha_ledger_mut() = Funded {
                    htlc_location: alpha_deploy_transaction.location,
                    deploy_transaction: alpha_deploy_transaction.transaction,
                    fund_transaction: alpha_fund_transaction.transaction,
                };
                *actor_state.beta_ledger_mut() = Funded {
                    htlc_location: beta_deploy_transaction.location,
                    deploy_transaction: beta_deploy_transaction.transaction,
                    fund_transaction: beta_fund_transaction.transaction,
                };
            }
            SS::AlphaFundedBetaRefunded(AlphaFundedBetaRefunded {
                beta_deploy_transaction,
                beta_fund_transaction,
                beta_refund_transaction,
                ..
            })
            | SS::Final(Final(SwapOutcome::BothRefunded {
                beta_deploy_transaction,
                beta_fund_transaction,
                alpha_or_beta_refunded_transaction: Either::Right(beta_refund_transaction),
                ..
            }))
            | SS::Final(Final(SwapOutcome::AlphaRedeemedBetaRefunded {
                beta_deploy_transaction,
                beta_fund_transaction,
                redeemed_or_refunded_transaction: Either::Right(beta_refund_transaction),
                ..
            })) => {
                *actor_state.beta_ledger_mut() = Refunded {
                    htlc_location: beta_deploy_transaction.location,
                    deploy_transaction: beta_deploy_transaction.transaction,
                    fund_transaction: beta_fund_transaction.transaction,
                    refund_transaction: beta_refund_transaction.transaction,
                }
            }
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                alpha_deploy_transaction,
                alpha_fund_transaction,
                alpha_refund_transaction,
                ..
            })
            | SS::Final(Final(SwapOutcome::AlphaRefunded {
                alpha_deploy_transaction,
                alpha_fund_transaction,
                alpha_refund_transaction,
                ..
            }))
            | SS::Final(Final(SwapOutcome::BothRefunded {
                alpha_deploy_transaction,
                alpha_fund_transaction,
                alpha_or_beta_refunded_transaction: Either::Left(alpha_refund_transaction),
                ..
            }))
            | SS::Final(Final(SwapOutcome::AlphaRefundedBetaRedeemed {
                alpha_deploy_transaction,
                alpha_fund_transaction,
                refunded_or_redeemed_transaction: Either::Left(alpha_refund_transaction),
                ..
            })) => {
                *actor_state.alpha_ledger_mut() = Refunded {
                    htlc_location: alpha_deploy_transaction.location,
                    deploy_transaction: alpha_deploy_transaction.transaction,
                    fund_transaction: alpha_fund_transaction.transaction,
                    refund_transaction: alpha_refund_transaction.transaction,
                }
            }
            SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                beta_deploy_transaction,
                beta_fund_transaction,
                beta_redeem_transaction,
                ..
            })
            | SS::Final(Final(SwapOutcome::BothRedeemed {
                beta_deploy_transaction,
                beta_fund_transaction,
                alpha_or_beta_redeemed_transaction: Either::Right(beta_redeem_transaction),
                ..
            }))
            | SS::Final(Final(SwapOutcome::AlphaRefundedBetaRedeemed {
                beta_deploy_transaction,
                beta_fund_transaction,
                refunded_or_redeemed_transaction: Either::Right(beta_redeem_transaction),
                ..
            })) => {
                *actor_state.beta_ledger_mut() = Redeemed {
                    htlc_location: beta_deploy_transaction.location,
                    deploy_transaction: beta_deploy_transaction.transaction,
                    fund_transaction: beta_fund_transaction.transaction,
                    redeem_transaction: beta_redeem_transaction.transaction,
                };
                actor_state.set_secret(beta_redeem_transaction.secret);
            }
            SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                alpha_deploy_transaction,
                alpha_fund_transaction,
                alpha_redeem_transaction,
                ..
            })
            | SS::Final(Final(SwapOutcome::AlphaRedeemed {
                alpha_deploy_transaction,
                alpha_fund_transaction,
                alpha_redeem_transaction,
                ..
            }))
            | SS::Final(Final(SwapOutcome::BothRedeemed {
                alpha_deploy_transaction,
                alpha_fund_transaction,
                alpha_or_beta_redeemed_transaction: Either::Left(alpha_redeem_transaction),
                ..
            }))
            | SS::Final(Final(SwapOutcome::AlphaRedeemedBetaRefunded {
                alpha_deploy_transaction,
                alpha_fund_transaction,
                redeemed_or_refunded_transaction: Either::Left(alpha_redeem_transaction),
                ..
            })) => {
                *actor_state.alpha_ledger_mut() = Redeemed {
                    htlc_location: alpha_deploy_transaction.location,
                    deploy_transaction: alpha_deploy_transaction.transaction,
                    fund_transaction: alpha_fund_transaction.transaction,
                    redeem_transaction: alpha_redeem_transaction.transaction,
                };
                actor_state.set_secret(alpha_redeem_transaction.secret);
            }
            SS::Error(ErrorState(e)) => {
                error!("Internal failure: {:?}", e);
                return;
            }
        }

        self.insert(key, actor_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        seed::Seed,
        swap_protocols::{
            ledger::{Bitcoin, Ethereum},
            rfc003::{alice, messages::Request, Secret, Timestamp},
        },
    };
    use bitcoin_support::BitcoinQuantity;
    use ethereum_support::EtherQuantity;
    use spectral::prelude::*;
    use std::sync::Arc;

    #[test]
    fn insert_and_get_state() {
        let state_store = InMemoryStateStore::default();
        let request = Request {
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger_refund_identity: secp256k1_support::KeyPair::from_secret_key_slice(
                &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                    .unwrap(),
            )
            .unwrap()
            .into(),
            beta_ledger_redeem_identity: "8457037fcd80a8650c4692d7fcfc1d0a96b92867"
                .parse()
                .unwrap(),
            alpha_expiry: Timestamp::from(2000000000),
            beta_expiry: Timestamp::from(2000000000),
            secret_hash: Secret::from(*b"hello world, you are beautiful!!").hash(),
        };
        let id = SwapId::default();
        let seed = Seed::from(*b"hello world, you are beautiful!!");
        let secret_source = Arc::new(seed.swap_seed(id));
        let state = alice::State::new(request, secret_source);

        state_store.insert::<alice::State<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>>(
            id,
            state.clone(),
        );

        let res = state_store
            .get::<alice::State<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>>(id)
            .unwrap();
        assert_that(&res).contains_value(state);
    }
}
