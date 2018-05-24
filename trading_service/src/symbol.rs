use std::fmt;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Symbol(pub String); // Expected format: BTC-LTC

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
