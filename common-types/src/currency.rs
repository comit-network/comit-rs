use std::fmt::Display;
use std::fmt;

#[derive(PartialEq, Debug)]
pub enum Currency {
    BTC,
    ETH,
    OTHER(String),
}

impl<'a> From<&'a str> for Currency {
    fn from(string: &str) -> Self {
        match string {
            "BTC" => Currency::BTC,
            "ETH" => Currency::ETH,
            _ => Currency::OTHER(string.to_string()),
        }
    }
}

impl Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &Currency::OTHER(ref value) => write!(f, "{}", value),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn can_deserialize_arbitrary_currency() {
        let foobar = Currency::from("FOOBAR");

        assert_eq!(foobar, Currency::OTHER("FOOBAR".to_string()))
    }
}
