use crate::{bitcoin, config, Result};

const ESTIMATE_FEE_TARGET: u32 = 3;

#[derive(Debug)]
pub struct Fee {
    config: config::BitcoinFeeStrategy,
    max_fee: bitcoin::Amount,
    client: bitcoin::Client,
}

impl Fee {
    pub fn new(
        config: config::BitcoinFeeStrategy,
        max_btc_fee: bitcoin::Amount,
        client: bitcoin::Client,
    ) -> Self {
        Self {
            config,
            max_fee: max_btc_fee,
            client,
        }
    }

    pub async fn sat_per_byte(&self) -> Result<bitcoin::Amount> {
        use crate::config::BitcoinFeeStrategy::*;
        match self.config {
            SatsPerByte(amount) => Ok(amount),
            BitcoindEstimateSmartfee(mode) => self
                .client
                .estimate_smart_fee(ESTIMATE_FEE_TARGET, Some(mode.into()))
                .await
                .map(|res| res.fee_rate),
        }
    }

    pub fn max_fee(&self) -> bitcoin::Amount {
        self.max_fee
    }
}

#[cfg(test)]
impl crate::StaticStub for Fee {
    fn static_stub() -> Self {
        Self {
            config: Default::default(),
            max_fee: bitcoin::Amount::ZERO,
            client: bitcoin::Client::new("http://example.com/".parse().unwrap()), // Not used
        }
    }
}
