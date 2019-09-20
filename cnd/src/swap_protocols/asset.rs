use crate::http_api::asset::FromHttpAsset;
use bitcoin_support::Amount;
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
    + Ord
{
}

impl Asset for Amount {}

impl Asset for EtherQuantity {}

impl Asset for Erc20Token {}

#[derive(Clone, Derivative, PartialEq)]
#[derivative(Debug = "transparent")]
pub enum AssetKind {
    Bitcoin(Amount),
    Ether(EtherQuantity),
    Erc20(Erc20Token),
    Unknown(String),
}

impl From<Amount> for AssetKind {
    fn from(amount: Amount) -> Self {
        AssetKind::Bitcoin(amount)
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
    use std::cmp::Ordering::*;

    #[test]
    fn test_bitcoin_quantity_cmp() {
        let quantity_1_btc = Amount::from_btc(1.0).unwrap();
        let quantity_10_btc = Amount::from_btc(10.0).unwrap();

        assert_that(&quantity_1_btc.cmp(&quantity_10_btc)).is_equal_to(Less);
        assert_that(&quantity_1_btc.cmp(&quantity_1_btc)).is_equal_to(Equal);
        assert_that(&quantity_10_btc.cmp(&quantity_1_btc)).is_equal_to(Greater);
    }

    #[test]
    fn test_ether_quantity_cmp() {
        let quantity_1_eth = EtherQuantity::from_eth(1.0);
        let quantity_10_eth = EtherQuantity::from_eth(10.0);

        assert_that(&quantity_1_eth.cmp(&quantity_10_eth)).is_equal_to(Less);
        assert_that(&quantity_1_eth.cmp(&quantity_1_eth)).is_equal_to(Equal);
        assert_that(&quantity_10_eth.cmp(&quantity_1_eth)).is_equal_to(Greater);
    }

    #[test]
    fn test_erc20_quantity_cmp() {
        let quantity_1_pay = Erc20Token::new(
            "B97048628DB6B661D4C2aA833e95Dbe1A905B280".parse().unwrap(),
            Erc20Quantity(U256::from(1u64)),
        );
        let quantity_10_pay = Erc20Token::new(
            "B97048628DB6B661D4C2aA833e95Dbe1A905B280".parse().unwrap(),
            Erc20Quantity(U256::from(10u64)),
        );

        assert_that(&quantity_1_pay.cmp(&quantity_10_pay)).is_equal_to(Less);
        assert_that(&quantity_1_pay.cmp(&quantity_1_pay)).is_equal_to(Equal);
        assert_that(&quantity_10_pay.cmp(&quantity_1_pay)).is_equal_to(Greater);
    }

    #[test]
    fn test_different_erc20_quantity_cmp() {
        let quantity_1_pay =
            Erc20Token::new(Address::repeat_byte(1), Erc20Quantity(U256::from(1u64)));
        let quantity_10_pay =
            Erc20Token::new(Address::repeat_byte(2), Erc20Quantity(U256::from(10u64)));

        assert_that(&quantity_1_pay.cmp(&quantity_10_pay)).is_equal_to(Less);
        assert_that(&quantity_1_pay.cmp(&quantity_1_pay)).is_equal_to(Equal);
        assert_that(&quantity_10_pay.cmp(&quantity_1_pay)).is_equal_to(Greater);
    }
}
