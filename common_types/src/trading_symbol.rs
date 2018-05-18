use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::de;
use serde::de::Visitor;
use std::fmt;

#[derive(PartialEq, Debug)]
#[allow(non_camel_case_types)]
pub enum TradingSymbol {
    ETH_BTC,
    UNKNOWN(String),
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
        write!(formatter, "a trading symbol (BTC:ETH)")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "ETH:BTC" => Ok(TradingSymbol::ETH_BTC),
            _ => Ok(TradingSymbol::UNKNOWN(value.to_string())),
        }
    }
}

impl Serialize for TradingSymbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let serialized_symbol = match self {
            &TradingSymbol::ETH_BTC => "ETH:BTC",
            &TradingSymbol::UNKNOWN(ref string) => string.as_str(),
        };

        serializer.serialize_str(serialized_symbol)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    extern crate serde_json;

    #[test]
    fn serializes_correctly() {
        let symbol = TradingSymbol::ETH_BTC;

        let serialized_symbol = serde_json::to_string(&symbol).unwrap();

        assert_eq!(serialized_symbol, r#""ETH:BTC""#)
    }

    #[test]
    fn deserializes_correctly() {
        let serialized_symbol = r#""ETH:BTC""#;

        let symbol = serde_json::from_str::<TradingSymbol>(serialized_symbol).unwrap();

        assert_eq!(symbol, TradingSymbol::ETH_BTC)
    }

}
