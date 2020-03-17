use crate::swap_protocols::rfc003::{
    events::{Deployed, Funded, Redeemed, Refunded},
    Secret,
};
use serde::Serialize;
use strum_macros::EnumDiscriminants;

#[derive(Clone, Debug, PartialEq, EnumDiscriminants)]
#[strum_discriminants(
    name(HtlcState),
    derive(Serialize, Display),
    serde(rename_all = "SCREAMING_SNAKE_CASE")
)]
pub enum LedgerState<A, H, T> {
    NotDeployed,
    Deployed {
        htlc_location: H,
        deploy_transaction: T,
    },
    Funded {
        htlc_location: H,
        deploy_transaction: T,
        fund_transaction: T,
        asset: A,
    },
    IncorrectlyFunded {
        htlc_location: H,
        deploy_transaction: T,
        fund_transaction: T,
        asset: A,
    },
    Redeemed {
        htlc_location: H,
        deploy_transaction: T,
        fund_transaction: T,
        redeem_transaction: T,
        asset: A,
        secret: Secret,
    },
    Refunded {
        htlc_location: H,
        deploy_transaction: T,
        fund_transaction: T,
        refund_transaction: T,
        asset: A,
    },
}

impl<A, H, T> LedgerState<A, H, T> {
    pub fn transition_to_deployed(&mut self, deployed: Deployed<H, T>) {
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

    pub fn transition_to_funded(&mut self, funded: Funded<A, T>) {
        match std::mem::replace(self, LedgerState::NotDeployed) {
            LedgerState::Deployed {
                deploy_transaction,
                htlc_location,
            } => match funded {
                Funded::Correctly { asset, transaction } => {
                    *self = LedgerState::Funded {
                        deploy_transaction,
                        htlc_location,
                        fund_transaction: transaction,
                        asset,
                    }
                }
                Funded::Incorrectly { asset, transaction } => {
                    *self = LedgerState::IncorrectlyFunded {
                        deploy_transaction,
                        htlc_location,
                        fund_transaction: transaction,
                        asset,
                    }
                }
            },
            other => panic!("expected state Deployed, got {}", HtlcState::from(other)),
        }
    }

    pub fn transition_to_redeemed(&mut self, redeemed: Redeemed<T>) {
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

    pub fn transition_to_refunded(&mut self, refunded: Refunded<T>) {
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
            3 => HtlcState::IncorrectlyFunded,
            4 => HtlcState::Redeemed,
            5 => HtlcState::Refunded,
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
