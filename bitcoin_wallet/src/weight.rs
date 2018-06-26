// TODO: contribute this to rust_bitcoin so it is returned by Transaction::get_weight
#[derive(Debug, PartialEq)]
pub struct Weight(u64);

impl Weight {
    pub fn to_virtual_bytes(&self) -> u64 {
        self.0 / 4
    }
}

impl From<Weight> for u64 {
    fn from(weight: Weight) -> u64 {
        weight.0
    }
}

impl From<u64> for Weight {
    fn from(value: u64) -> Self {
        Weight(value)
    }
}
