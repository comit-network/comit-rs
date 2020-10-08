use ::bitcoin::Amount;

#[derive(Debug, Clone)]
pub struct BitcoinFees(Amount);

impl BitcoinFees {
    pub fn new(per_vbyte_rate: Amount) -> Self {
        Self(per_vbyte_rate)
    }
    pub async fn get_per_vbyte_rate(&self) -> anyhow::Result<Amount> {
        Ok(self.0)
    }
}
