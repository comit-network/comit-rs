use ethereum_support::{EtherQuantity, U256};
use lazy_static::lazy_static;
use std::{f64, str::FromStr};

lazy_static! {
    static ref WEI_IN_ETHEREUM: U256 = U256::from((10u64).pow(18));
}

#[test]
fn display_ethereum() {
    assert_eq!(EtherQuantity::from_eth(9000.0).to_string(), "9000 ETH");
}

#[test]
fn a_ethereum_is_a_quintillion_wei() {
    assert_eq!(
        EtherQuantity::from_eth(2.0).wei(),
        U256::from(2_000_000_000_000_000_000u64) // 2 quintillion
    )
}

#[test]
fn from_eth_works_when_resulting_wei_cant_fit_in_u64() {
    assert_eq!(
        EtherQuantity::from_eth(9001.0).wei(),
        U256::from(9001u64) * *WEI_IN_ETHEREUM
    )
}

#[test]
fn from_fractional_ethereum_converts_to_correct_wei() {
    assert_eq!(
        EtherQuantity::from_eth(0.000_000_001).wei(),
        U256::from(1_000_000_000)
    )
}

#[test]
fn ether_quantity_from_str() {
    assert_eq!(
        EtherQuantity::from_str("1.000000001").unwrap().wei(),
        U256::from(1_000_000_001_000_000_000u64)
    )
}

#[test]
fn ether_quantity_back_into_f64() {
    assert!(EtherQuantity::from_eth(0.1234).ethereum() - 0.1234f64 < f64::EPSILON)
}

#[test]
fn fractional_ethereum_format() {
    assert_eq!(EtherQuantity::from_eth(0.1234).to_string(), "0.1234 ETH")
}

#[test]
fn whole_ethereum_format() {
    assert_eq!(EtherQuantity::from_eth(12.0).to_string(), "12 ETH");
}

#[test]
fn ethereum_with_small_fraction_format() {
    assert_eq!(
        EtherQuantity::from_str("1234.00000100")
            .unwrap()
            .to_string(),
        "1234.000001 ETH"
    )
}

#[test]
fn one_hundren_ethereum_format() {
    assert_eq!(
        EtherQuantity::from_str("100").unwrap().to_string(),
        "100 ETH"
    )
}

#[test]
fn serialize_ether_quantity() {
    let quantity = EtherQuantity::from_eth(1.0);
    let quantity_str = serde_json::to_string(&quantity).unwrap();
    assert_eq!(quantity_str, "\"1000000000000000000\"");
}

#[test]
fn deserialize_ether_quantity() {
    let quantity_str = "\"1000000000000000000\"";
    let quantity = serde_json::from_str::<EtherQuantity>(quantity_str).unwrap();
    assert_eq!(quantity, EtherQuantity::from_eth(1.0));
}
