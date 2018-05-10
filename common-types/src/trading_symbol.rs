use regex::Regex;
use serde::de;
use serde::de::Unexpected;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use std::fmt;
use super::*;

#[derive(PartialEq, Debug)]
pub struct TradingSymbol(String, String);

impl<'de> Deserialize<'de> for TradingSymbol {
    fn deserialize<D>(deserializer: D) -> Result<TradingSymbol, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(TradingSymbolVisitor)
    }
}

lazy_static! {
    static ref TRADING_SYMBOL_REGEX: Regex = Regex::new(r"(?x)
    (?P<first>[A-Z0-9]+)  # the first part
    :                     # separator
    (?P<second>[A-Z0-9]+) # the second part
    ")
    .unwrap();
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
        match TRADING_SYMBOL_REGEX.captures(value) {
            Some(groups) => {
                let first = &groups["first"];
                let second = &groups["second"];

                Ok(TradingSymbol(first.to_string(), second.to_string()))
            }
            None => Err(de::Error::invalid_value(Unexpected::Str(value), &self)),
        }
    }
}

impl Serialize for TradingSymbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let serialized_symbol = format!("{}:{}", self.0, self.1);

        serializer.serialize_str(&serialized_symbol)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    extern crate serde_json;

    #[test]
    fn serializes_correctly() {
        let symbol = TradingSymbol("ETH".to_string(), "BTC".to_string());

        let serialized_symbol = serde_json::to_string(&symbol).unwrap();

        assert_eq!(serialized_symbol, r#""ETH:BTC""#)
    }

    #[test]
    fn deserializes_correctly() {
        let serialized_symbol = r#""ETH:BTC""#;

        let symbol = serde_json::from_str::<TradingSymbol>(serialized_symbol).unwrap();

        assert_eq!(symbol, TradingSymbol("ETH".to_string(), "BTC".to_string()))
    }

}
