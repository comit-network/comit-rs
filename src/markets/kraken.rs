use crate::markets;
use crate::markets::TradingPair;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::de::Error;
use serde::Deserialize;
use std::convert::TryFrom;

/// Fetch OHLC (open-high-low-close) data
/// More info here: https://www.kraken.com/features/api
pub async fn get_ohlc(trading_pair: TradingPair) -> anyhow::Result<markets::Ohlc> {
    let trading_pair_code = get_trading_pair_code(trading_pair);

    // Interval used when fetching the ohlc data from Kraken.
    // The data returned will contain segments according to the interval.
    // The highest frequency time interval for OHLC data is 1 minute, possible values:
    // 1 (default), 5, 15, 30, 60, 240, 1440, 10080, 21600
    let time_interval = 30;
    // By passing in a timestamp far in the futrue we reduce the API to return only the last OHLC value
    let since = 2_147_483_647;

    let request_url = format!("https://api.kraken.com/0/public/OHLC?pair={trading_pair}&interval={time_interval}&since={since}",
                              trading_pair = trading_pair_code,
                              time_interval = time_interval,
                              since = since,
    );

    let response = reqwest::get(&request_url)
        .await?
        .json::<OhlcResponse>()
        .await?;

    let ohlc = response.result.xbtdai;
    let ohlc = ohlc
        .last()
        .ok_or_else(|| anyhow::Error::msg("No data returned from Kraken OHLC API"))?;

    Ok(markets::Ohlc {
        high: ohlc.high,
        low: ohlc.low,
        vwap: ohlc.vwap,
        timestamp: ohlc.timestamp,
        trading_pair,
    })
}

#[derive(Deserialize)]
struct OhlcResponse {
    result: XbtDaiRates,
}

#[derive(Deserialize)]
struct XbtDaiRates {
    #[serde(rename = "XBTDAI")]
    xbtdai: Vec<Ohlc>,
}

#[derive(Deserialize)]
struct RateItems(Vec<RateItem>);

#[derive(Deserialize)]
#[serde(untagged)]
enum RateItem {
    String(String),
    Number(u32),
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "RateItems")]
struct Ohlc {
    timestamp: DateTime<Utc>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    vwap: f64, // volume weighted average price
    volume: f64,
    count: u32,
}

impl TryFrom<RateItems> for Ohlc {
    type Error = serde_json::Error;

    fn try_from(value: RateItems) -> Result<Self, Self::Error> {
        let (timestamp, open, high, low, close, vwap, volume, count) = match value.0.as_slice() {
            [RateItem::Number(timestamp), RateItem::String(open), RateItem::String(high), RateItem::String(low), RateItem::String(close), RateItem::String(vwap), RateItem::String(volume), RateItem::Number(count)] => {
                (timestamp, open, high, low, close, vwap, volume, count)
            }
            _ => return Err(serde_json::Error::custom("OHLC array malformed")),
        };

        Ok(Ohlc {
            timestamp: DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp(*timestamp as i64, 0),
                Utc,
            ),
            open: open.parse::<f64>().map_err(serde_json::Error::custom)?,
            high: high.parse::<f64>().map_err(serde_json::Error::custom)?,
            low: low.parse::<f64>().map_err(serde_json::Error::custom)?,
            close: close.parse::<f64>().map_err(serde_json::Error::custom)?,
            vwap: vwap.parse::<f64>().map_err(serde_json::Error::custom)?,
            volume: volume.parse::<f64>().map_err(serde_json::Error::custom)?,
            count: *count,
        })
    }
}

fn get_trading_pair_code(trading_pair: TradingPair) -> String {
    match trading_pair {
        TradingPair::BtcDai => "XBTDAI".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OHLC_EXAMPLE_DATA: &str = r#"{
  "error": [],
  "result": {
    "XBTDAI": [
      [
        1581508800,
        "10354.3",
        "10412.1",
        "10317.1",
        "10317.1",
        "10367.3",
        "0.25537510",
        6
      ],
      [
        1581523200,
        "10317.1",
        "10371.6",
        "10317.1",
        "10320.8",
        "10363.0",
        "0.32213808",
        24
      ]
    ],
    "last": 1591848000
  }
}"#;

    #[test]
    fn given_ohlc_example_data_deserializes_correctly() {
        serde_json::from_str::<OhlcResponse>(OHLC_EXAMPLE_DATA).unwrap();
    }
}
