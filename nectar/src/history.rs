use crate::fs::ensure_directory_exists;
use anyhow::Result;
use chrono::{DateTime, Utc};
use csv::*;
use libp2p::PeerId;
use num::BigUint;
use serde::{Serialize, Serializer};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

#[derive(Debug)]
pub struct History {
    writer: Writer<File>,
}

impl History {
    pub fn new(path: &Path) -> Result<History> {
        ensure_directory_exists(&path)?;

        let writer = if path.exists() {
            let file = OpenOptions::new().append(true).open(path)?;
            WriterBuilder::new().has_headers(false).from_writer(file)
        } else {
            Writer::from_path(path)?
        };

        Ok(History { writer })
    }

    pub fn write(&mut self, trade: Trade) -> anyhow::Result<()> {
        self.writer.serialize(trade)?;
        self.writer.flush()?;
        Ok(())
    }
}

/// All the information to write in the CVS file per trade
// If you change this then you need to think about versioning
#[derive(Debug, Clone, Serialize)]
pub struct Trade {
    /// When the trade was taken and accepted
    #[serde(serialize_with = "datetime_rfc3339")]
    pub utc_start_timestamp: DateTime<Utc>,
    /// When the last transaction (redeem or refund) was seen (can be changed to
    /// confirmed in the future)
    #[serde(serialize_with = "datetime_rfc3339")]
    pub utc_final_timestamp: DateTime<Utc>,
    /// The symbol of the base currency
    pub base_symbol: Symbol,
    /// The symbol of the quote currency
    pub quote_symbol: Symbol,
    /// The position of the trade from the user's point of view (note: Sell =
    /// sell the base)
    pub position: Position,
    /// The base currency traded amount in the most precise unit (e.g. Satoshi)
    /// Note: it does not include fees
    #[serde(serialize_with = "biguint_string")]
    pub base_precise_amount: BigUint,
    /// The quote currency traded amount in the most precise unit (e.g. attodai)
    /// Note: it does not include fees
    #[serde(serialize_with = "biguint_string")]
    pub quote_precise_amount: BigUint,
    /// the Peer id of the counterpart/taker
    #[serde(serialize_with = "peerid_string")]
    pub peer: PeerId,
    // TODO: Add fees?
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Symbol {
    Btc,
    Dai,
}

#[derive(Debug, Copy, Clone, Serialize)]
pub enum Position {
    Buy,
    Sell,
}

fn biguint_string<S>(
    value: &BigUint,
    serializer: S,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn peerid_string<S>(
    value: &PeerId,
    serializer: S,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn datetime_rfc3339<S>(
    value: &DateTime<Utc>,
    serializer: S,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_rfc3339())
}

#[cfg(test)]
impl crate::StaticStub for PeerId {
    fn static_stub() -> Self {
        use std::str::FromStr;

        PeerId::from_str("QmUJF1AzhjUfDU1ifzkyuHy26SCnNHbPaVHpX1WYxYYgZg").unwrap()
    }
}

#[cfg(test)]
impl Trade {
    fn new_1() -> Self {
        use std::str::FromStr;

        Trade {
            utc_start_timestamp: "2020-07-10T17:48:26.123+10:00".parse().unwrap(),
            utc_final_timestamp: "2020-07-10T18:48:26.456+10:00".parse().unwrap(),
            base_symbol: Symbol::Btc,
            quote_symbol: Symbol::Dai,
            position: Position::Buy,
            base_precise_amount: 1_000_000u64.into(),
            quote_precise_amount: BigUint::from_str("99_000_000_000_000_000_000").unwrap(),
            peer: PeerId::from_str("QmUJF1AzhjUfDU1ifzkyuHy26SCnNHbPaVHpX1WYxYYgZg").unwrap(),
        }
    }

    fn new_2() -> Self {
        use std::str::FromStr;

        Trade {
            utc_start_timestamp: "2020-07-11T12:00:00.789+10:00".parse().unwrap(),
            utc_final_timestamp: "2020-07-11T13:00:00.000+10:00".parse().unwrap(),
            base_symbol: Symbol::Btc,
            quote_symbol: Symbol::Dai,
            position: Position::Sell,
            base_precise_amount: 20_000_000u64.into(),
            quote_precise_amount: BigUint::from_str("2_012_340_000_000_000_000_000").unwrap(),
            peer: PeerId::from_str("QmccqkBDb51kDJzvC26EdXprvFhcsLPNmYQRPMwDMmEUhK").unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::TempDir;

    #[test]
    fn write_two_trades_with_headers() {
        let temp_file = TempDir::new().unwrap().path().join("history.csv");
        let trade_1 = Trade::new_1();
        let trade_2 = Trade::new_2();
        let mut history = History::new(&temp_file).unwrap();

        history.write(trade_1).unwrap();
        history.write(trade_2).unwrap();

        let mut file = File::open(temp_file).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        let expected_contents = "utc_start_timestamp,utc_final_timestamp,base_symbol,quote_symbol,position,base_precise_amount,quote_precise_amount,peer
2020-07-10T07:48:26.123+00:00,2020-07-10T08:48:26.456+00:00,BTC,DAI,Buy,1000000,99000000000000000000,QmUJF1AzhjUfDU1ifzkyuHy26SCnNHbPaVHpX1WYxYYgZg
2020-07-11T02:00:00.789+00:00,2020-07-11T03:00:00+00:00,BTC,DAI,Sell,20000000,2012340000000000000000,QmccqkBDb51kDJzvC26EdXprvFhcsLPNmYQRPMwDMmEUhK
";

        assert_eq!(contents, expected_contents);
    }

    #[test]
    fn re_use_existing_file_without_losing_data_or_re_writing_headers() {
        let temp_file = TempDir::new().unwrap().path().join("history.csv");
        let trade_1 = Trade::new_1();
        let trade_2 = Trade::new_2();
        let mut history = History::new(&temp_file).unwrap();

        history.write(trade_1).unwrap();

        // Re-instantiate history to test re-usage of an existing file
        let mut history = History::new(&temp_file).unwrap();

        history.write(trade_2).unwrap();

        let mut file = File::open(temp_file).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        let expected_contents = "utc_start_timestamp,utc_final_timestamp,base_symbol,quote_symbol,position,base_precise_amount,quote_precise_amount,peer
2020-07-10T07:48:26.123+00:00,2020-07-10T08:48:26.456+00:00,BTC,DAI,Buy,1000000,99000000000000000000,QmUJF1AzhjUfDU1ifzkyuHy26SCnNHbPaVHpX1WYxYYgZg
2020-07-11T02:00:00.789+00:00,2020-07-11T03:00:00+00:00,BTC,DAI,Sell,20000000,2012340000000000000000,QmccqkBDb51kDJzvC26EdXprvFhcsLPNmYQRPMwDMmEUhK
";

        assert_eq!(contents, expected_contents);
    }
}
