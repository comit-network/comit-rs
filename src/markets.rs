mod kraken;
use chrono::{DateTime, Utc};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, strum_macros::Display)]
pub enum TradingPair {
    BtcDai,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Rate {
    trading_pair: TradingPair,
    position: Position,
    rate: f64,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, strum_macros::Display)]
pub enum Position {
    BUY,
    SELL,
}

pub struct Ohlc {
    high: f64,
    low: f64,
    vwap: f64,
    timestamp: DateTime<Utc>,
    trading_pair: TradingPair,
}

impl Ohlc {
    fn to_rate(&self, position: Position) -> anyhow::Result<Rate> {
        let rate = if self.vwap == 0.0 {
            let precision = 0.000_000_000_000_001;
            if (self.high - self.low).abs() > precision {
                anyhow::bail!("OHLC high and low value are not the same even though there were no trades recorded (vwap 0).")
            }
            self.high
        } else {
            self.vwap
        };

        let trading_pair = self.trading_pair;
        let timestamp = self.timestamp;

        match position {
            Position::BUY => Ok(Rate {
                trading_pair,
                position,
                rate: 1f64 / rate,
                timestamp,
            }),
            Position::SELL => Ok(Rate {
                trading_pair,
                position,
                rate,
                timestamp,
            }),
        }
    }
}

// Only Kraken atm, can be extended to more markets later (and then choosing best rate or whatnot)
pub async fn get_rate(trading_pair: TradingPair, position: Position) -> anyhow::Result<Rate> {
    kraken::get_ohlc(trading_pair).await?.to_rate(position)
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error(
    "no rate found for trading pair {trading_pair} on position {position}"
)]
pub struct NoRateFound {
    trading_pair: TradingPair,
    position: Position,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn ohlc_with_vwap() -> Ohlc {
        // These are actual values from Kraken
        Ohlc {
            timestamp: Utc::now(),
            high: 9825.1,
            low: 9791.0,
            vwap: 9806.7,
            trading_pair: TradingPair::BtcDai,
        }
    }

    fn ohlc_without_vwap() -> Ohlc {
        Ohlc {
            timestamp: Utc::now(),
            high: 9000.0,
            low: 9000.0,
            vwap: 0.0,
            trading_pair: TradingPair::BtcDai,
        }
    }

    fn ohlc_without_vwap_different_high_low() -> Ohlc {
        Ohlc {
            timestamp: Utc::now(),
            high: 10000.0,
            low: 9000.0,
            vwap: 0.0,
            trading_pair: TradingPair::BtcDai,
        }
    }

    #[test]
    fn given_sell_order_ohlc_data_without_vwap_use_high_low() {
        let ohlc_without_vwap = ohlc_without_vwap();

        let rate = ohlc_without_vwap.to_rate(Position::SELL).unwrap();

        assert_eq!(rate.trading_pair, ohlc_without_vwap.trading_pair);
        assert_eq!(rate.position, Position::SELL);
        assert_eq!(rate.rate, 9000.0);
    }

    #[test]
    fn given_buy_order_ohlc_data_without_vwap_use_high_low() {
        let rate = ohlc_without_vwap().to_rate(Position::BUY).unwrap();
        let expected_rate = 1.0 / 9000.0;
        assert_eq!(rate.rate, expected_rate);
    }

    #[test]
    fn given_sell_order_ohlc_data_with_vwap_use_vwap() {
        let rate = ohlc_with_vwap().to_rate(Position::SELL).unwrap();

        assert_eq!(rate.rate, 9806.7);
    }

    #[test]
    fn given_buy_order_ohlc_data_with_vwap_use_vwap() {
        let rate = ohlc_with_vwap().to_rate(Position::BUY).unwrap();

        assert_eq!(rate.rate, 1.0 / 9806.7);
    }

    #[test]
    fn given_buy_order_ohlc_data_without_vwap_and_different_high_low() {
        let rate = ohlc_without_vwap_different_high_low().to_rate(Position::BUY);
        assert!(rate.is_err());
    }
}
