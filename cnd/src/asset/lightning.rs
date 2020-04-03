use bitcoin::{util::amount::Denomination, Amount};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Lightning(Amount);

impl Lightning {
    pub fn from_sat(sat: u64) -> Lightning {
        Lightning(Amount::from_sat(sat))
    }

    pub fn as_sat(self) -> u64 {
        Amount::as_sat(self.0)
    }

    pub fn to_le_bytes(self) -> [u8; 8] {
        self.0.as_sat().to_le_bytes()
    }
}

impl From<Lightning> for Amount {
    fn from(lightning: Lightning) -> Self {
        Amount::from_sat(lightning.as_sat())
    }
}

impl fmt::Display for Lightning {
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
            asset::Lightning::from_sat(900_000_000_000).to_string(),
            "9000.00000000 BTC"
        );
    }
}
