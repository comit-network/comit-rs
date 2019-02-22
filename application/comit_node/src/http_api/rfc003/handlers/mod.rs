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

use serde::{Serialize, Serializer};

#[derive(Debug)]
pub struct Http<I>(I);

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
