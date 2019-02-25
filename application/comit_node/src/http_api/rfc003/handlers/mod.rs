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

macro_rules! _count {
    () => (0usize);
    ($x:tt $($xs:tt)*) => (1usize + _count!($($xs)*));
}

macro_rules! impl_serialize_http {
    ($type:ty $(:= $name:tt)? { $($field_name:tt $(=> $field_value:ident)?),* }) => {
        impl Serialize for Http<$type> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let params = _count!($($name)*);
                let mut state = serializer.serialize_struct("", 1 + params)?;

                #[allow(unused_variables)]
                let name = stringify!($type);
                $(let name = $name;)?
                state.serialize_field("name", name)?;

                $(
                  state.serialize_field($field_name, &(self.0)$(.$field_value)?)?;
                )*

                state.end()
            }
        }
    };
}

impl_serialize_http!(Bitcoin { "network" => network });
impl_serialize_http!(Ethereum { "network" => network });

impl_serialize_http!(BitcoinQuantity := "Bitcoin" { "quantity" });
impl_serialize_http!(EtherQuantity := "Ether" { "quantity" });
impl_serialize_http!(Erc20Token := "ERC20" { "quantity" => quantity, "token_contract" => token_contract });

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
