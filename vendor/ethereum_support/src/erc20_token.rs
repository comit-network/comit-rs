use crate::{erc20_quantity::Erc20Quantity, web3::types::Address};
use std::fmt;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Erc20Token {
    token_contract: Address,
    quantity: Erc20Quantity,
}

impl fmt::Display for Erc20Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.quantity)
    }
}

impl Erc20Token {
    pub fn new(token_contract: Address, quantity: Erc20Quantity) -> Self {
        Erc20Token {
            token_contract,
            quantity,
        }
    }

    pub fn token_contract(&self) -> Address {
        self.token_contract
    }

    pub fn quantity(&self) -> Erc20Quantity {
        self.quantity
    }
}
