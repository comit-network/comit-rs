mod get_action;
mod get_swap;
mod get_swaps;
mod post_action;
mod post_swap;

pub use self::{
    get_action::{handle_get_action, GetAction, GetActionQueryParams},
    get_swap::handle_get_swap,
    get_swaps::handle_get_swaps,
    post_action::{handle_post_action, PostAction},
    post_swap::{handle_post_swap, SwapRequestBodyKind},
};

use crate::swap_protocols::ledger::{Bitcoin, Ethereum};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Token, EtherQuantity};
use serde::{ser::SerializeStruct, Serialize, Serializer};

#[derive(Debug)]
pub struct Http<I>(pub I);

impl Serialize for Http<Bitcoin> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("", 2)?;
        state.serialize_field("name", "Bitcoin")?;
        state.serialize_field("network", &self.0.network)?;
        state.end()
    }
}

impl Serialize for Http<Ethereum> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("", 2)?;
        state.serialize_field("name", "Ethereum")?;
        state.serialize_field("network", &self.0.network)?;
        state.end()
    }
}

impl Serialize for Http<BitcoinQuantity> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("", 2)?;
        state.serialize_field("name", "Bitcoin")?;
        state.serialize_field("quantity", &self.0)?;
        state.end()
    }
}

impl Serialize for Http<EtherQuantity> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("", 2)?;
        state.serialize_field("name", "Ether")?;
        state.serialize_field("quantity", &self.0)?;
        state.end()
    }
}

impl Serialize for Http<Erc20Token> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("", 3)?;
        state.serialize_field("name", "ERC20")?;
        state.serialize_field("quantity", &self.0.quantity())?;
        state.serialize_field("token_contract", &self.0.token_contract())?;
        state.end()
    }
}

impl Serialize for Http<bitcoin_support::Transaction> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self.0.txid()))
    }
}

impl Serialize for Http<ethereum_support::Transaction> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.hash.serialize(serializer)
    }
}

impl Serialize for Http<bitcoin_support::OutPoint> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl Serialize for Http<ethereum_support::Address> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}
