use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

#[derive(PartialEq, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum TradingSymbol {
    ETH_BTC,
    ETH_LN,
    UNKNOWN(String),
}

impl fmt::Display for TradingSymbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let symbol = match *self {
            TradingSymbol::ETH_BTC => "ETH-BTC",
            TradingSymbol::ETH_LN => "ETH-LN",
            TradingSymbol::UNKNOWN(ref string) => string.as_str(),
        };

        write!(f, "{}", symbol)
    }
}

impl<'de> Deserialize<'de> for TradingSymbol {
    fn deserialize<D>(deserializer: D) -> Result<TradingSymbol, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(TradingSymbolVisitor)
    }
}

struct TradingSymbolVisitor;

impl<'de> Visitor<'de> for TradingSymbolVisitor {
    type Value = TradingSymbol;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a trading symbol (BTC-ETH)")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "ETH-BTC" => Ok(TradingSymbol::ETH_BTC),
            "ETH-LN" => Ok(TradingSymbol::ETH_LN),
            _ => Ok(TradingSymbol::UNKNOWN(value.to_string())),
        }
    }
}

impl Serialize for TradingSymbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(format!("{}", self).as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate serde_json;

    #[test]
    fn serializes_eth_btc_correctly() {
        let symbol = TradingSymbol::ETH_BTC;

        let serialized_symbol = serde_json::to_string(&symbol).unwrap();

        assert_eq!(serialized_symbol, r#""ETH-BTC""#)
    }

    #[test]
    fn deserializes_eth_btc_correctly() {
        let serialized_symbol = r#""ETH-BTC""#;

        let symbol = serde_json::from_str::<TradingSymbol>(serialized_symbol).unwrap();

        assert_eq!(symbol, TradingSymbol::ETH_BTC)
    }

    #[test]
    fn serializes_eth_ln_correctly() {
        let symbol = TradingSymbol::ETH_LN;

        let serialized_symbol = serde_json::to_string(&symbol).unwrap();

        assert_eq!(serialized_symbol, r#""ETH-LN""#)
    }

    #[test]
    fn deserializes_eth_ln_correctly() {
        let serialized_symbol = r#""ETH-LN""#;

        let symbol = serde_json::from_str::<TradingSymbol>(serialized_symbol).unwrap();

        assert_eq!(symbol, TradingSymbol::ETH_LN)
    }

}
