extern crate bitcoin_support;
extern crate serde_json;

use bitcoin_support::BitcoinQuantity;
use std::str::FromStr;

#[test]
fn hundred_million_sats_is_a_bitcoin() {
    assert_eq!(BitcoinQuantity::from_satoshi(100_000_000).bitcoin(), 1.0);
}

#[test]
fn a_bitcoin_is_a_hundred_million_sats() {
    assert_eq!(BitcoinQuantity::from_bitcoin(1.0).satoshi(), 100_000_000);
}

#[test]
fn a_bitcoin_as_string_is_a_hundred_million_sats() {
    assert_eq!(
        BitcoinQuantity::from_str("1.00000001").unwrap(),
        BitcoinQuantity::from_bitcoin(1.00000001)
    )
}

#[test]
fn bitcoin_with_small_fraction_format() {
    assert_eq!(
        format!("{}", BitcoinQuantity::from_str("1234.00000100").unwrap()),
        "1234.000001 BTC"
    )
}

#[test]
fn one_hundred_bitcoin_format() {
    assert_eq!(
        format!("{}", BitcoinQuantity::from_str("100").unwrap()),
        "100 BTC"
    )
}

#[test]
fn display_bitcoin() {
    assert_eq!(format!("{}", BitcoinQuantity::from_bitcoin(42.0)), "42 BTC");
    assert_eq!(
        format!("{}", BitcoinQuantity::from_satoshi(200_000_000)),
        "2 BTC"
    );
}

#[test]
fn serialize_bitcoin_quantity() {
    let quantity = BitcoinQuantity::from_satoshi(100_000_000);
    assert_eq!(serde_json::to_string(&quantity).unwrap(), "\"100000000\"");
}

#[test]
fn deserialize_bitcoin_quantity() {
    let quantity = serde_json::from_str::<BitcoinQuantity>("\"100000000\"").unwrap();
    assert_eq!(quantity, BitcoinQuantity::from_satoshi(100_000_000))
}
