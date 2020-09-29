use crate::{bitcoin, bitcoin::Amount, config, Result};
use anyhow::Context;

const ESTIMATE_FEE_TARGET: u32 = 3;

#[derive(Clone, Debug)]
pub struct Fee {
    config: config::BitcoinFeeStrategy,
    max_fee: bitcoin::Amount,
    client: bitcoin::Client,
}

impl Fee {
    // TODO: Improve this API, the client is not needed
    // if we use the static fee
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

    pub async fn kbyte_rate(&self) -> Result<Amount> {
        let byte_rate = self.byte_rate().await?;
        byte_rate
            .checked_mul(1000)
            .context("Could not mul byte rate")
    }

    pub async fn byte_rate(&self) -> Result<Amount> {
        use crate::config::BitcoinFeeStrategy::*;
        match self.config {
            SatsPerByte(amount) => Ok(amount),
            BitcoindEstimateSmartfee(mode) => {
                let kbyte_rate = self
                    .client
                    .estimate_smart_fee(ESTIMATE_FEE_TARGET, Some(mode.into()))
                    .await
                    .map(|res| res.kbyte_rate)?;

                // Return rate per byte
                kbyte_rate
                    .checked_div(1000)
                    .context("Could not div kbyte rate")
            }
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
