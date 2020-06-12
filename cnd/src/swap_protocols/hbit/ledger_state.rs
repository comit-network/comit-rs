use crate::{
    asset, htlc_location,
    swap_protocols::{
        hbit::events::{Deployed, Funded, Redeemed, Refunded},
        Secret,
    },
    transaction,
};
use std::fmt::{self, Display};

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum LedgerState {
    NotDeployed,
    Deployed {
        htlc_location: htlc_location::Bitcoin,
        deploy_transaction: transaction::Bitcoin,
    },
    Funded {
        htlc_location: htlc_location::Bitcoin,
        deploy_transaction: transaction::Bitcoin,
        fund_transaction: transaction::Bitcoin,
        asset: asset::Bitcoin,
    },
    IncorrectlyFunded {
        htlc_location: htlc_location::Bitcoin,
        deploy_transaction: transaction::Bitcoin,
        fund_transaction: transaction::Bitcoin,
        asset: asset::Bitcoin,
    },
    Redeemed {
        htlc_location: htlc_location::Bitcoin,
        deploy_transaction: transaction::Bitcoin,
        fund_transaction: transaction::Bitcoin,
        redeem_transaction: transaction::Bitcoin,
        asset: asset::Bitcoin,
        secret: Secret,
    },
    Refunded {
        htlc_location: htlc_location::Bitcoin,
        deploy_transaction: transaction::Bitcoin,
        fund_transaction: transaction::Bitcoin,
        refund_transaction: transaction::Bitcoin,
        asset: asset::Bitcoin,
    },
}

impl Display for LedgerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            LedgerState::NotDeployed { .. } => "NotDeployed".to_string(),
            LedgerState::Deployed { .. } => "Deployed".to_string(),
            LedgerState::Funded { .. } => "Funded".to_string(),
            LedgerState::IncorrectlyFunded { .. } => "IncorrectlyFunded".to_string(),
            LedgerState::Redeemed { .. } => "Redeemed".to_string(),
            LedgerState::Refunded { .. } => "Refunded".to_string(),
        };

        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HtlcState {
    NotDeployed,
    Deployed,
    Funded,
    IncorrectlyFunded,
    Redeemed,
    Refunded,
}

impl LedgerState {
    pub fn transition_to_deployed(&mut self, deployed: Deployed) {
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
            other => panic!("expected state NotDeployed, got {}", other),
        }
    }

    pub fn transition_to_funded(&mut self, funded: Funded) {
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
            other => panic!("expected state Deployed, got {}", other),
        }
    }

    pub fn transition_to_redeemed(&mut self, redeemed: Redeemed) {
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
            other => panic!("expected state Funded, got {}", other),
        }
    }

    pub fn transition_to_refunded(&mut self, refunded: Refunded) {
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
            other => panic!("expected state Funded or IncorrectlyFunded, got {}", other),
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
