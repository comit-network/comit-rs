use crate::{
    asset::Asset,
    swap_protocols::rfc003::{
        events::{Deployed, Funded, Redeemed, Refunded},
        ledger::Ledger,
        Secret,
    },
};
use serde::Serialize;
use strum_macros::EnumDiscriminants;

#[derive(Clone, Debug, PartialEq, EnumDiscriminants)]
#[strum_discriminants(
    name(HtlcState),
    derive(Serialize, Display),
    serde(rename_all = "SCREAMING_SNAKE_CASE")
)]
pub enum LedgerState<L: Ledger, A: Asset> {
    NotDeployed,
    Deployed {
        htlc_location: L::HtlcLocation,
        deploy_transaction: L::Transaction,
    },
    Funded {
        htlc_location: L::HtlcLocation,
        deploy_transaction: L::Transaction,
        fund_transaction: L::Transaction,
        asset: A,
    },
    Redeemed {
        htlc_location: L::HtlcLocation,
        deploy_transaction: L::Transaction,
        fund_transaction: L::Transaction,
        redeem_transaction: L::Transaction,
        asset: A,
        secret: Secret,
    },
    Refunded {
        htlc_location: L::HtlcLocation,
        deploy_transaction: L::Transaction,
        fund_transaction: L::Transaction,
        refund_transaction: L::Transaction,
        asset: A,
    },
    IncorrectlyFunded {
        htlc_location: L::HtlcLocation,
        deploy_transaction: L::Transaction,
        fund_transaction: L::Transaction,
        asset: A,
    },
}

impl<L: Ledger, A: Asset> LedgerState<L, A> {
    pub fn transition_to_deployed(&mut self, deployed: Deployed<L>) {
        let Deployed {
            transaction,
            location,
        } = deployed;

        match std::mem::replace(self, LedgerState::NotDeployed) {
            LedgerState::NotDeployed => {
                *self = LedgerState::Deployed {
                    deploy_transaction: transaction,
                    htlc_location: location,
                }
            }
            other => panic!("expected state NotDeployed, got {}", HtlcState::from(other)),
        }
    }

    pub fn transition_to_funded(&mut self, funded: Funded<L::Transaction, A>) {
        let Funded { transaction, asset } = funded;

        match std::mem::replace(self, LedgerState::NotDeployed) {
            LedgerState::Deployed {
                deploy_transaction,
                htlc_location,
            } => {
                *self = LedgerState::Funded {
                    deploy_transaction,
                    htlc_location,
                    fund_transaction: transaction,
                    asset,
                }
            }
            other => panic!("expected state Deployed, got {}", HtlcState::from(other)),
        }
    }

    pub fn transition_to_incorrectly_funded(&mut self, funded: Funded<L::Transaction, A>) {
        let Funded { transaction, asset } = funded;

        match std::mem::replace(self, LedgerState::NotDeployed) {
            LedgerState::Deployed {
                deploy_transaction,
                htlc_location,
            } => {
                *self = LedgerState::IncorrectlyFunded {
                    deploy_transaction,
                    htlc_location,
                    fund_transaction: transaction,
                    asset,
                }
            }
            other => panic!("expected state Deployed, got {}", HtlcState::from(other)),
        }
    }

    pub fn transition_to_redeemed(&mut self, redeemed: Redeemed<L>) {
        let Redeemed {
            transaction,
            secret,
        } = redeemed;

        match std::mem::replace(self, LedgerState::NotDeployed) {
            LedgerState::Funded {
                deploy_transaction,
                htlc_location,
                asset,
                fund_transaction,
            } => {
                *self = LedgerState::Redeemed {
                    deploy_transaction,
                    htlc_location,
                    fund_transaction,
                    redeem_transaction: transaction,
                    asset,
                    secret,
                }
            }
            other => panic!("expected state Funded, got {}", HtlcState::from(other)),
        }
    }

    pub fn transition_to_refunded(&mut self, refunded: Refunded<L>) {
        let Refunded { transaction } = refunded;

        match std::mem::replace(self, LedgerState::NotDeployed) {
            LedgerState::Funded {
                deploy_transaction,
                htlc_location,
                asset,
                fund_transaction,
            }
            | LedgerState::IncorrectlyFunded {
                deploy_transaction,
                htlc_location,
                asset,
                fund_transaction,
            } => {
                *self = LedgerState::Refunded {
                    deploy_transaction,
                    htlc_location,
                    fund_transaction,
                    refund_transaction: transaction,
                    asset,
                }
            }
            other => panic!(
                "expected state Funded or IncorrectlyFunded, got {}",
                HtlcState::from(other)
            ),
        }
    }
}

impl Default for HtlcState {
    fn default() -> Self {
        HtlcState::NotDeployed
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for HtlcState {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        match g.next_u32() % 6 {
            0 => HtlcState::NotDeployed,
            1 => HtlcState::Deployed,
            2 => HtlcState::Funded,
            3 => HtlcState::Redeemed,
            4 => HtlcState::Refunded,
            5 => HtlcState::IncorrectlyFunded,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn not_deployed_serializes_correctly_to_json() {
        let state = HtlcState::NotDeployed;
        let serialized = serde_json::to_string(&state).unwrap();
        assert_eq!(serialized, r#""NOT_DEPLOYED""#);
    }
}
