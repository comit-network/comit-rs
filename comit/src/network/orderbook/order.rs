use crate::{asset, identity, ledger};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderId(Uuid);

impl OrderId {
    pub fn random() -> OrderId {
        OrderId(Uuid::new_v4())
    }
}

impl Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for OrderId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::from_str(s)?;
        Ok(OrderId(uuid))
    }
}

/// Represents a "form" that contains all data for creating a new order.
#[derive(Debug)]
pub struct NewOrder {
    pub position: Position,
    pub bitcoin_amount: asset::Bitcoin,
    pub bitcoin_ledger: ledger::Bitcoin,
    pub bitcoin_absolute_expiry: u32,
    pub ethereum_amount: asset::Erc20Quantity,
    pub token_contract: identity::Ethereum,
    pub ethereum_ledger: ledger::Ethereum,
    pub ethereum_absolute_expiry: u32,
}

impl NewOrder {
    // TODO: I think this should live in the controller an be asserted before we
    // construct the type
    pub fn assert_valid_ledger_pair(&self) -> anyhow::Result<()> {
        let a = self.bitcoin_ledger;
        let b = self.ethereum_ledger;

        if ledger::is_valid_ledger_pair(a, b) {
            return Ok(());
        }
        Err(anyhow::anyhow!("invalid ledger pair {}/{}", a, b))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Order {
    pub id: OrderId,
    pub maker: PeerId,
    pub position: Position,
    pub bitcoin_amount: asset::Bitcoin,
    pub bitcoin_ledger: ledger::Bitcoin,
    pub bitcoin_absolute_expiry: u32,
    pub ethereum_amount: asset::Erc20Quantity,
    pub token_contract: identity::Ethereum,
    pub ethereum_ledger: ledger::Ethereum,
    pub ethereum_absolute_expiry: u32,
}

/// The position of the maker for this order. A BTC/DAI buy order,
/// also described as an order that buys the trading pair BTC/DAI,
/// means that the maker buys the base currency (in this case BTC) in
/// return for DAI. A sell order means that the maker sells BTC and
/// receives DAI.
///
/// Please note: we do not set the base currency to 1 and use rate
/// (i.e., quote currency) and amount as is commonly done in Forex
/// trading. We use the amounts of each currency to determine the
/// rate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Buy,
    Sell,
}
