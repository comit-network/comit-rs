use ::bitcoin::Amount;

#[derive(Debug, Clone)]
pub enum BitcoinFees {
    StaticPerVbyte(Amount),
    BlockCypher(block_cypher::Client),
}

impl BitcoinFees {
    pub fn static_rate(per_vbyte_rate: Amount) -> Self {
        Self::StaticPerVbyte(per_vbyte_rate)
    }

    pub fn block_cypher(url: url::Url, network: comit::Network) -> Self {
        let client = block_cypher::Client::new(url, network);
        Self::BlockCypher(client)
    }

    pub async fn get_per_vbyte_rate(&self) -> anyhow::Result<Amount> {
        match self {
            BitcoinFees::StaticPerVbyte(rate) => Ok(*rate),
            BitcoinFees::BlockCypher(client) => {
                let per_kb = client
                    .get_fee_per_kb_as_per_expiries_recommendation()
                    .await?;
                Ok(per_kb / 1000)
            }
        }
    }
}

mod block_cypher {
    use ::bitcoin::Amount;
    use anyhow::{Context, Result};
    use serde::Deserialize;

    #[derive(Debug, Clone)]
    pub struct Client {
        url: url::Url,
        mine_within_blocks: u8,
    }

    impl Client {
        /// Blockchain endpoint expected, see: https://www.blockcypher.com/dev/bitcoin/#blockchain
        pub fn new(url: url::Url, network: comit::Network) -> Self {
            let mine_within_blocks = comit::expiries::bitcoin_mine_within_blocks(network);
            Self {
                url,
                mine_within_blocks,
            }
        }

        pub async fn get_fee_per_kb_as_per_expiries_recommendation(&self) -> Result<Amount> {
            if self.mine_within_blocks < 3 {
                self.high_fee_per_kb().await
            } else if self.mine_within_blocks < 7 {
                self.medium_fee_per_kb().await
            } else {
                self.low_fee_per_kb().await
            }
        }

        async fn high_fee_per_kb(&self) -> Result<Amount> {
            let response = self.get().await?;

            let fee = response.high_fee_per_kb;

            tracing::debug!(
                "Blockcypher estimate a wait for confirmation within 1 to 2 blocks: {} per kilobyte",
                fee
            );

            Ok(fee)
        }

        async fn medium_fee_per_kb(&self) -> Result<Amount> {
            let response = self.get().await?;

            let fee = response.medium_fee_per_kb;

            tracing::debug!(
                "Blockcypher estimate a wait for confirmation within 3 to 6 blocks: {} per kilobyte",
                fee
            );

            Ok(fee)
        }

        async fn low_fee_per_kb(&self) -> Result<Amount> {
            let response = self.get().await?;
            let fee = response.low_fee_per_kb;

            tracing::debug!(
                "Blockcypher estimate a wait for confirmation within 7 blocks or more: {} per kilobyte",
                fee
            );

            Ok(fee)
        }

        async fn get(&self) -> Result<Response> {
            Ok(reqwest::get(self.url.clone())
                .await
                .with_context(|| format!("failed to perform GET request to {}", self.url))?
                .json()
                .await
                .context("failed to deserialize response as JSON into struct")?)
        }
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct Response {
        /// A rolling average of the fee (in satoshis) paid per kilobyte for
        /// transactions to be confirmed within 1 to 2 blocks.
        #[serde(with = "::bitcoin::util::amount::serde::as_sat")]
        pub high_fee_per_kb: Amount,
        /// A rolling average of the fee (in satoshis) paid per kilobyte for
        /// transactions to be confirmed within 3 to 6 blocks.
        #[serde(with = "::bitcoin::util::amount::serde::as_sat")]
        pub medium_fee_per_kb: Amount,
        /// A rolling average of the fee (in satoshis) paid per kilobyte for
        /// transactions to be confirmed in 7 or more blocks.
        #[serde(with = "::bitcoin::util::amount::serde::as_sat")]
        pub low_fee_per_kb: Amount,
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn deserialize_blockcypher() {
            let str = r#"{
  "name": "BTC.main",
  "height": 651754,
  "hash": "00000000000000000003c3260ac52b81d2061c02bd3451b965953d51bf667fa1",
  "time": "2020-10-08T04:14:57.988028407Z",
  "latest_url": "https://api.blockcypher.com/v1/btc/main/blocks/00000000000000000003c3260ac52b81d2061c02bd3451b965953d51bf667fa1",
  "previous_hash": "0000000000000000000e3cb097583f3407219d78a1955a3a5c9f8f9d64ab4723",
  "previous_url": "https://api.blockcypher.com/v1/btc/main/blocks/0000000000000000000e3cb097583f3407219d78a1955a3a5c9f8f9d64ab4723",
  "peer_count": 1044,
  "unconfirmed_count": 3911,
  "high_fee_per_kb": 100112,
  "medium_fee_per_kb": 50836,
  "low_fee_per_kb": 39029,
  "last_fork_height": 650473,
  "last_fork_hash": "00000000000000000005ee10eff75a0db9620516e399db5767b084877473c5e0"
}"#;

            let value: Response = serde_json::from_str(str).unwrap();

            assert_eq!(value, Response {
                high_fee_per_kb: bitcoin::Amount::from_sat(100112),
                medium_fee_per_kb: Amount::from_sat(50836),
                low_fee_per_kb: Amount::from_sat(39029)
            })
        }
    }
}
