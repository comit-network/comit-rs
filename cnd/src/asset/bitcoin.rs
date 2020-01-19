use bitcoin::{util::amount::Denomination, Amount};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Bitcoin(Amount);

impl Bitcoin {
    pub fn from_sat(sat: u64) -> Bitcoin {
        Bitcoin(Amount::from_sat(sat))
    }

    pub fn as_sat(self) -> u64 {
        Amount::as_sat(self.0)
    }
}

impl From<Bitcoin> for Amount {
    fn from(bitcoin: Bitcoin) -> Self {
        Amount::from_sat(bitcoin.as_sat())
    }
}

impl fmt::Display for Bitcoin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let bitcoin = self.0.to_string_in(Denomination::Bitcoin);
        write!(f, "{} BTC", bitcoin)
    }
}

#[cfg(test)]
mod tests {
    use crate::asset;

    #[test]
    fn display_bitcoin() {
        assert_eq!(
            asset::Bitcoin::from_sat(900_000_000_000).to_string(),
            "9000.00000000 BTC"
        );
    }
}
