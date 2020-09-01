use crate::{
    asset, ethereum, halbit, hbit, herc20, http_api::amount::Amount, Role, Secret, SecretHash,
    Timestamp,
};
use comit::asset::Erc20Quantity;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase", tag = "protocol")]
pub enum Protocol {
    Hbit { asset: Amount },
    Herc20 { asset: Amount },
    Halbit { asset: Amount },
}

impl Protocol {
    pub fn hbit(btc: asset::Bitcoin) -> Self {
        Protocol::Hbit {
            asset: Amount::btc(btc),
        }
    }

    pub fn halbit(btc: asset::Bitcoin) -> Self {
        Protocol::Halbit {
            asset: Amount::btc(btc),
        }
    }

    pub fn herc20_dai(dai: Erc20Quantity) -> Self {
        Protocol::Herc20 {
            asset: Amount::dai(dai),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ActionName {
    Init,
    Deploy,
    Fund,
    Redeem,
    Refund,
}

pub trait Events {
    fn events(&self) -> Vec<SwapEvent>;
}

/// Get the underlying ledger used by the alpha protocol.
pub trait AlphaLedger {
    fn alpha_ledger(&self) -> Ledger;
}

/// Get the underlying ledger used by the beta protocol.
pub trait BetaLedger {
    fn beta_ledger(&self) -> Ledger;
}

/// Get the absolute expiry time for the alpha protocol.
pub trait AlphaAbsoluteExpiry {
    fn alpha_absolute_expiry(&self) -> Option<Timestamp>;
}

/// Get the absolute expiry time for the beta protocol.
pub trait BetaAbsoluteExpiry {
    fn beta_absolute_expiry(&self) -> Option<Timestamp>;
}

/// Ledgers we currently support swaps on top of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ledger {
    Bitcoin,
    Ethereum,
}

pub trait GetRole {
    fn get_role(&self) -> Role;
}

pub trait AlphaProtocol {
    fn alpha_protocol(&self) -> Protocol;
}

pub trait BetaProtocol {
    fn beta_protocol(&self) -> Protocol;
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum SwapEvent {
    HbitFunded { tx: bitcoin::Txid },
    HbitIncorrectlyFunded { tx: bitcoin::Txid },
    HbitRedeemed { tx: bitcoin::Txid },
    HbitRefunded { tx: bitcoin::Txid },
    Herc20Deployed { tx: ethereum::Hash },
    Herc20Funded { tx: ethereum::Hash },
    Herc20IncorrectlyFunded { tx: ethereum::Hash },
    Herc20Redeemed { tx: ethereum::Hash },
    Herc20Refunded { tx: ethereum::Hash },

    // TODO: Seriously reconsider this naming + the whole halbit protocol design in general. The
    // event-based design here should allow us to name this whatever and hence make it more
    // descriptive.
    HalbitFunded,
    HalbitIncorrectlyFunded,
    HalbitRedeemed,
    HalbitRefunded,
}

impl From<&herc20::State> for Vec<SwapEvent> {
    fn from(state: &herc20::State) -> Self {
        match state {
            herc20::State::None => vec![],
            herc20::State::Deployed {
                deploy_transaction, ..
            } => vec![SwapEvent::Herc20Deployed {
                tx: deploy_transaction.hash,
            }],
            herc20::State::Funded {
                deploy_transaction,
                fund_transaction,
                ..
            } => vec![
                SwapEvent::Herc20Deployed {
                    tx: deploy_transaction.hash,
                },
                SwapEvent::Herc20Funded {
                    tx: fund_transaction.hash,
                },
            ],
            herc20::State::IncorrectlyFunded {
                deploy_transaction,
                fund_transaction,
                ..
            } => vec![
                SwapEvent::Herc20Deployed {
                    tx: deploy_transaction.hash,
                },
                SwapEvent::Herc20IncorrectlyFunded {
                    tx: fund_transaction.hash,
                },
            ],
            herc20::State::Redeemed {
                deploy_transaction,
                fund_transaction,
                redeem_transaction,
                ..
            } => vec![
                SwapEvent::Herc20Deployed {
                    tx: deploy_transaction.hash,
                },
                SwapEvent::Herc20Funded {
                    tx: fund_transaction.hash,
                },
                SwapEvent::Herc20Redeemed {
                    tx: redeem_transaction.hash,
                },
            ],
            herc20::State::Refunded {
                deploy_transaction,
                fund_transaction,
                refund_transaction,
                ..
            } => vec![
                SwapEvent::Herc20Deployed {
                    tx: deploy_transaction.hash,
                },
                SwapEvent::Herc20Funded {
                    tx: fund_transaction.hash,
                },
                SwapEvent::Herc20Refunded {
                    tx: refund_transaction.hash,
                },
            ],
        }
    }
}

impl From<&hbit::State> for Vec<SwapEvent> {
    fn from(state: &hbit::State) -> Self {
        match state {
            hbit::State::None => vec![],
            hbit::State::Funded {
                fund_transaction, ..
            } => vec![SwapEvent::HbitFunded {
                tx: fund_transaction.txid(),
            }],
            hbit::State::IncorrectlyFunded {
                fund_transaction, ..
            } => vec![
                SwapEvent::HbitFunded {
                    tx: fund_transaction.txid(),
                },
                SwapEvent::HbitIncorrectlyFunded {
                    tx: fund_transaction.txid(),
                },
            ],
            hbit::State::Redeemed {
                fund_transaction,
                redeem_transaction,
                ..
            } => vec![
                SwapEvent::HbitFunded {
                    tx: fund_transaction.txid(),
                },
                SwapEvent::HbitRedeemed {
                    tx: redeem_transaction.txid(),
                },
            ],
            hbit::State::Refunded {
                fund_transaction,
                refund_transaction,
                ..
            } => vec![
                SwapEvent::HbitFunded {
                    tx: fund_transaction.txid(),
                },
                SwapEvent::HbitRefunded {
                    tx: refund_transaction.txid(),
                },
            ],
        }
    }
}

impl From<&halbit::State> for Vec<SwapEvent> {
    fn from(state: &halbit::State) -> Self {
        match state {
            halbit::State::None => vec![],
            halbit::State::Opened(_) => vec![],
            halbit::State::Accepted(_) => vec![SwapEvent::HalbitFunded],
            halbit::State::Settled(_) => vec![SwapEvent::HalbitFunded, SwapEvent::HalbitRedeemed],
            halbit::State::Cancelled(_) => vec![SwapEvent::HalbitFunded, SwapEvent::HalbitRefunded],
        }
    }
}

#[derive(Clone, Debug)]
pub enum AliceSwap<AC, BC, AF, BF> {
    Created {
        alpha_created: AC,
        beta_created: BC,
    },
    Finalized {
        alpha_finalized: AF,
        beta_finalized: BF,
        secret: Secret,
    },
}

#[derive(Clone, Debug)]
pub enum BobSwap<AC, BC, AF, BF> {
    Created {
        alpha_created: AC,
        beta_created: BC,
    },
    Finalized {
        alpha_finalized: AF,
        beta_finalized: BF,
        secret_hash: SecretHash,
    },
}

impl<AC, BC, AF, BF> GetRole for AliceSwap<AC, BC, AF, BF> {
    fn get_role(&self) -> Role {
        Role::Alice
    }
}

impl<AC, BC, AF, BF> GetRole for BobSwap<AC, BC, AF, BF> {
    fn get_role(&self) -> Role {
        Role::Bob
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::ethereum::FromWei;

    #[test]
    fn hbit_protocol_serializes_correctly() {
        let protocol = Protocol::hbit(asset::Bitcoin::from_sat(10_000));

        let result = serde_json::to_string_pretty(&protocol).unwrap();

        assert_eq!(
            result,
            r#"{
  "protocol": "hbit",
  "asset": {
    "currency": "BTC",
    "value": "10000",
    "decimals": 8
  }
}"#
        )
    }

    #[test]
    fn halbit_protocol_serializes_correctly() {
        let protocol = Protocol::halbit(asset::Bitcoin::from_sat(10_000));

        let result = serde_json::to_string_pretty(&protocol).unwrap();

        assert_eq!(
            result,
            r#"{
  "protocol": "halbit",
  "asset": {
    "currency": "BTC",
    "value": "10000",
    "decimals": 8
  }
}"#
        )
    }

    #[test]
    fn herc20_protocol_serializes_correctly() {
        let protocol = Protocol::herc20_dai(Erc20Quantity::from_wei(1_000_000_000_000_000u64));

        let result = serde_json::to_string_pretty(&protocol).unwrap();

        assert_eq!(
            result,
            r#"{
  "protocol": "herc20",
  "asset": {
    "currency": "DAI",
    "value": "1000000000000000",
    "decimals": 18
  }
}"#
        )
    }
}
