use crate::{bitcoin, bitcoin::Amount, config, Result};
use anyhow::Context;

#[derive(Clone, Debug)]
pub struct Fee {
    config: config::Bitcoin,
    client: bitcoin::Client,
}

impl Fee {
    // TODO: Improve this API, the client is not needed
    // if we use the static fee
    pub fn new(config: config::Bitcoin, client: bitcoin::Client) -> Self {
        Self { config, client }
    }

    pub async fn kvbyte_rate(&self, block_target: u8) -> Result<Amount> {
        let rate = self.vbyte_rate(block_target).await?;
        rate.checked_mul(1000).context("Could not mul byte rate")
    }

    pub async fn vbyte_rate(&self, block_target: u8) -> Result<Amount> {
        use crate::config::BitcoinFees::*;
        match self.config.fees {
            SatsPerByte(fee) => Ok(fee),
            BitcoindEstimateSmartfee { mode, .. } => {
                let mine_within_blocks = block_target as u32;

                let kvbyte_rate = self
                    .client
                    .estimate_smart_fee(mine_within_blocks, Some(mode.into()))
                    .await
                    .map(|res| res.kbyte_rate)?;

                // Return rate per byte
                kvbyte_rate
                    .checked_div(1000)
                    .context("Could not div kbyte rate")
            }
        }
    }

    pub fn max_tx_fee(&self) -> bitcoin::Amount {
        self.config.fees.max_tx_fee()
    }
}

#[cfg(test)]
impl crate::StaticStub for Fee {
    fn static_stub() -> Self {
        Self {
            config: crate::StaticStub::static_stub(),
            client: bitcoin::Client::new("http://example.com/".parse().unwrap()), // Not used
        }
    }
}
