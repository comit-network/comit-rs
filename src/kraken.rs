use crate::kraken::btc_dai::{MidMarketRate, Rate};
use serde::de::Error;
use serde::Deserialize;
use std::convert::TryFrom;

/// Fetch Ticker data
/// More info here: https://www.kraken.com/features/api
pub async fn get_mid_market_rate() -> anyhow::Result<MidMarketRate> {
    let rate = reqwest::get("https://api.kraken.com/0/public/Ticker?pair=XBTDAI")
        .await?
        .json::<TickerResponse>()
        .await
        .map(|response| response.result.xbtdai)?;

    Ok(MidMarketRate::from_kraken(rate))
}

#[derive(Deserialize)]
struct TickerResponse {
    result: Ticker,
}

#[derive(Deserialize)]
struct Ticker {
    #[serde(rename = "XBTDAI")]
    xbtdai: Rate,
}

#[derive(Deserialize)]
struct TickerData {
    #[serde(rename = "a")]
    ask: Vec<String>,
    #[serde(rename = "b")]
    bid: Vec<String>,
}

impl TryFrom<TickerData> for Rate {
    type Error = serde_json::Error;

    fn try_from(value: TickerData) -> Result<Self, Self::Error> {
        let ask_price = value
            .ask
            .first()
            .ok_or_else(|| serde_json::Error::custom("no ask price"))?;
        let bid_price = value
            .bid
            .first()
            .ok_or_else(|| serde_json::Error::custom("no bid price"))?;

        Ok(Rate {
            ask: ask_price
                .parse::<f64>()
                .map_err(serde_json::Error::custom)?,
            bid: bid_price
                .parse::<f64>()
                .map_err(serde_json::Error::custom)?,
        })
    }
}

pub mod btc_dai {
    use super::*;
    use chrono::{DateTime, Utc};

    #[derive(Debug, Copy, Clone, PartialEq)]
    pub struct MidMarketRate {
        value: f64,
        timestamp: DateTime<Utc>,
    }

    impl MidMarketRate {
        pub fn from_kraken(rate: Rate) -> Self {
            MidMarketRate {
                value: (rate.bid + rate.ask) / 2f64,
                timestamp: Utc::now(),
            }
        }
    }

    #[derive(Debug, Deserialize)]
    #[serde(try_from = "TickerData")]
    pub struct Rate {
        pub ask: f64,
        pub bid: f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICKER_EXAMPLE: &str = r#"{
    "error": [],
    "result": {
        "XBTDAI": {
            "a": [
                "9489.50000",
                "1",
                "1.000"
            ],
            "b": [
                "9462.70000",
                "1",
                "1.000"
            ],
            "c": [
                "9496.50000",
                "0.00220253"
            ],
            "v": [
                "0.19793959",
                "0.55769847"
            ],
            "p": [
                "9583.44469",
                "9593.15707"
            ],
            "t": [
                12,
                22
            ],
            "l": [
                "9496.50000",
                "9496.50000"
            ],
            "h": [
                "9594.90000",
                "9616.10000"
            ],
            "o": "9562.30000"
        }
    }
}"#;

    #[test]
    fn given_ticker_example_data_deserializes_correctly() {
        serde_json::from_str::<TickerResponse>(TICKER_EXAMPLE).unwrap();
    }
}
