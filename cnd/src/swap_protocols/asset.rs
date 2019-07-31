use crate::http_api::asset::FromHttpAsset;
use bitcoin_support::BitcoinQuantity;
use derivative::Derivative;
use ethereum_support::{Erc20Token, EtherQuantity};
use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

pub trait Asset:
    Clone
    + Copy
    + Debug
    + Display
    + Send
    + Sync
    + 'static
    + PartialEq
    + Eq
    + Hash
    + FromHttpAsset
    + Into<AssetKind>
{
    fn compare_to(&self, other: &Self) -> i8;
}

impl Asset for BitcoinQuantity {
    fn compare_to(&self, other: &BitcoinQuantity) -> i8 {
        if self < other {
            -1
        }
        if self > other {
            1
        }
        0
    }
}
impl Asset for EtherQuantity {
    fn compare_to(&self, other: &EtherQuantity) -> i8 {
        if self < other {
            -1
        }
        if self > other {
            1
        }
        0
    }
}
impl Asset for Erc20Token {
    fn compare_to(&self, other: &Erc20Token) -> i8 {
        if self.quantity < other.quantity {
            -1
        }
        if self.quantity > other.quantity {
            1
        }
        0
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug = "transparent")]
pub enum AssetKind {
    Bitcoin(BitcoinQuantity),
    Ether(EtherQuantity),
    Erc20(Erc20Token),
    Unknown(String),
}

impl From<BitcoinQuantity> for AssetKind {
    fn from(quantity: BitcoinQuantity) -> Self {
        AssetKind::Bitcoin(quantity)
    }
}

impl From<EtherQuantity> for AssetKind {
    fn from(quantity: EtherQuantity) -> Self {
        AssetKind::Ether(quantity)
    }
}

impl From<Erc20Token> for AssetKind {
    fn from(quantity: Erc20Token) -> Self {
        AssetKind::Erc20(quantity)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ethereum_support::{Address, Erc20Quantity, Erc20Token, U256};
    use spectral::*;

    #[test]
    fn test_bitcoin_quantity_compare_to() {
        let quantity_1_btc = BitcoinQuantity::from_bitcoin(1.0);
        let quantity_10_btc = BitcoinQuantity::from_bitcoin(10.0);

        assert_that(&quantity_1_btc.compare_to(&quantity_10_btc)).is_equal_to(-1);
        assert_that(&quantity_1_btc.compare_to(&quantity_1_btc)).is_equal_to(0);
        assert_that(&quantity_10_btc.compare_to(&quantity_1_btc)).is_equal_to(1);
    }

    #[test]
    fn test_ether_quantity_compare_to() {
        let quantity_1_eth = EtherQuantity::from_eth(1.0);
        let quantity_10_eth = EtherQuantity::from_eth(10.0);

        assert_that(&quantity_1_eth.compare_to(&quantity_10_eth)).is_equal_to(-1);
        assert_that(&quantity_1_eth.compare_to(&quantity_1_eth)).is_equal_to(0);
        assert_that(&quantity_10_eth.compare_to(&quantity_1_eth)).is_equal_to(1);
    }

    #[test]
    fn test_erc20_quantity_compare_to() {
        let quantity_1_pay = Erc20Token::new(
            Address::from("0xB97048628DB6B661D4C2aA833e95Dbe1A905B280"),
            Erc20Quantity(U256::from(1u64)),
        );
        let quantity_10_pay = Erc20Token::new(
            Address::from("0xB97048628DB6B661D4C2aA833e95Dbe1A905B280"),
            Erc20Quantity(U256::from(10u64)),
        );

        assert_that(&quantity_1_pay.compare_to(&quantity_10_pay)).is_equal_to(-1);
        assert_that(&quantity_1_pay.compare_to(&quantity_1_pay)).is_equal_to(0);
        assert_that(&quantity_10_pay.compare_to(&quantity_1_pay)).is_equal_to(1);
    }

    #[test]
    fn test_different_erc20_quantity_compare_to() {
        let quantity_1_pay = Erc20Token::new(
            Address::from("0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"),
            Erc20Quantity(U256::from(1u64)),
        );
        let quantity_10_pay = Erc20Token::new(
            Address::from("0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
            Erc20Quantity(U256::from(10u64)),
        );

        assert_that(&quantity_1_pay.compare_to(&quantity_10_pay)).is_equal_to(-1);
        assert_that(&quantity_1_pay.compare_to(&quantity_1_pay)).is_equal_to(0);
        assert_that(&quantity_10_pay.compare_to(&quantity_1_pay)).is_equal_to(1);
    }
}
