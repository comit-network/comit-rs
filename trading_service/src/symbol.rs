use std::fmt;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Symbol(pub String); // Expected format: LTC-BTC

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
