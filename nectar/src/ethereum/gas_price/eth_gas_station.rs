use crate::{ethereum::ether::Amount, Result};
use num::BigUint;
use serde::{de::Error, Deserialize, Deserializer};
use std::convert::TryFrom;
use time::Duration;

#[derive(Debug, Clone)]
pub struct Client {
    url: url::Url,
}

impl Client {
    pub fn new(url: url::Url) -> Self {
        Self { url }
    }

    pub async fn gas_price(&self) -> Result<Amount> {
        let response: Response = reqwest::get(self.url.clone())
            .await
            .map_err(ConnectionFailed)?
            .json()
            .await
            .map_err(DeserializationFailed)?;

        tracing::info!(
            "Eth Gas Station estimate a wait of {:?} for {} gwei gas price",
            response.safe_low_wait,
            response.safe_low
        );

        Ok(response.safe_low)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("connection error: {0}")]
pub struct ConnectionFailed(#[from] reqwest::Error);

#[derive(Debug, thiserror::Error)]
#[error("deserialization error: {0}")]
pub struct DeserializationFailed(#[from] reqwest::Error);

// TODO: Use the value that would satisfy
// comit::expiries::config::ETHEREUM_MINE_WITHIN_N_BLOCKS;
#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Response {
    /// Low gas price value that is safe
    #[serde(deserialize_with = "ether_tenx_gigawei")]
    pub safe_low: Amount,
    /// Estimated wait duration using safe low gas price
    #[serde(deserialize_with = "minute_duration")]
    pub safe_low_wait: Duration,
}

fn ether_tenx_gigawei<'de, D>(deserializer: D) -> Result<Amount, D::Error>
where
    D: Deserializer<'de>,
{
    let int = u64::deserialize(deserializer)?;
    let amount = BigUint::from(int);

    // We want wei from 10x gwei
    // Make it gwei: *1_000_000_000
    // But actually it's 10x gwei: /10
    let amount = amount * (1_000_000_000u64 / 10u64);

    // Accepts the amount is wei
    let amount = Amount::try_from(amount).map_err(D::Error::custom)?;
    Ok(amount)
}

fn minute_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let minutes = f64::deserialize(deserializer)?;
    let duration = Duration::seconds_f64(minutes * 60.0);
    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_response() {
        let str = r#"
{
  "fast": 780,
  "fastest": 860,
  "safeLow": 630,
  "average": 700,
  "block_time": 8.625,
  "blockNum": 10960796,
  "speed": 0.8947913750497876,
  "safeLowWait": 7.3,
  "avgWait": 0.8,
  "fastWait": 0.4,
  "fastestWait": 0.3,
  "gasPriceRange": {
    "4": 143.8,
    "6": 143.8,
    "8": 143.8,
    "10": 143.8,
    "20": 143.8,
    "30": 143.8,
    "40": 143.8,
    "50": 143.8,
    "60": 143.8,
    "70": 143.8,
    "80": 143.8,
    "90": 143.8,
    "100": 143.8,
    "110": 143.8,
    "120": 143.8,
    "130": 143.8,
    "140": 143.8,
    "150": 143.8,
    "160": 143.8,
    "170": 143.8,
    "180": 143.8,
    "190": 143.8,
    "200": 143.8,
    "220": 143.8,
    "240": 143.8,
    "260": 143.8,
    "280": 143.8,
    "300": 143.8,
    "320": 143.8,
    "340": 143.8,
    "360": 143.8,
    "380": 143.8,
    "400": 143.8,
    "420": 143.8,
    "440": 143.8,
    "460": 143.8,
    "480": 143.8,
    "500": 143.8,
    "520": 143.8,
    "540": 9.9,
    "560": 9.6,
    "580": 9.3,
    "600": 8,
    "620": 7.8,
    "630": 7.3,
    "640": 6.9,
    "660": 5.9,
    "680": 5.6,
    "700": 0.8,
    "720": 0.6,
    "740": 0.5,
    "760": 0.4,
    "780": 0.4,
    "800": 0.3,
    "820": 0.3,
    "840": 0.3,
    "860": 0.3
  }
}
        "#;

        let value: Response = serde_json::from_str(str).unwrap();

        assert_eq!(value, Response {
            safe_low: Amount::from_ether_str("0.000000063").expect("eth value"), /* 63 gwei = 0.000000063 eth */
            safe_low_wait: Duration::seconds(438) // 7.3 minutes = 438 seconds
        })
    }
}
