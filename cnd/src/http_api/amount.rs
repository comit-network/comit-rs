use crate::{asset, asset::Erc20Quantity};
use comit::{Price, Quantity};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "currency")]
pub enum Amount {
    #[serde(rename = "BTC")]
    Bitcoin {
        #[serde(with = "asset::bitcoin::sats_as_string")]
        value: asset::Bitcoin,
        decimals: u8,
    },
    #[serde(rename = "DAI")]
    Dai { value: Erc20Quantity, decimals: u8 },
}

impl Amount {
    pub fn btc(value: asset::Bitcoin) -> Self {
        Amount::Bitcoin { value, decimals: 8 }
    }

    pub fn dai(value: Erc20Quantity) -> Self {
        Amount::Dai {
            value,
            decimals: 18,
        }
    }
}

impl From<Quantity<asset::Bitcoin>> for Amount {
    fn from(quantity: Quantity<asset::Bitcoin>) -> Self {
        Amount::btc(quantity.to_inner())
    }
}

impl From<Price<asset::Bitcoin, Erc20Quantity>> for Amount {
    fn from(price: Price<asset::Bitcoin, Erc20Quantity>) -> Self {
        Amount::dai(price.wei_per_btc())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btc_amount_serializes_properly() {
        let amount = Amount::btc(asset::Bitcoin::from_sat(100000000));

        let string = serde_json::to_string(&amount).unwrap();

        assert_eq!(
            string,
            r#"{"currency":"BTC","value":"100000000","decimals":8}"#
        )
    }

    #[test]
    fn dai_amount_serializes_properly() {
        let amount =
            Amount::dai(Erc20Quantity::from_wei_dec_str("9000000000000000000000").unwrap());

        let string = serde_json::to_string(&amount).unwrap();

        assert_eq!(
            string,
            r#"{"currency":"DAI","value":"9000000000000000000000","decimals":18}"#
        )
    }
}
