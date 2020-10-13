//! A suite of tests that ensures the serialization format of the types we use
//! to interact with the database. Changing the format needs to be a conscious
//! activity that involves migration scripts to migrate old data. These tests
//! make sure we don't change the format accidentally!

use crate::{
    identity,
    storage::db::wrapper_types::{Erc20Amount, Ether, Satoshis},
    LockProtocol, SecretHash,
};
use comit::{ledger, Position};
use std::{fmt, str::FromStr};

#[test]
fn bitcoin_network() {
    roundtrip_test::<ledger::Bitcoin>("mainnet");
    roundtrip_test::<ledger::Bitcoin>("testnet");
    roundtrip_test::<ledger::Bitcoin>("regtest");
}

#[test]
fn ether() {
    roundtrip_test::<Ether>("1000000000000000");
}

#[test]
fn erc20_amount() {
    roundtrip_test::<Erc20Amount>("1000000000000000");
}

#[test]
fn bitcoin_amount() {
    roundtrip_test::<Satoshis>("100000000000");
}

#[test]
fn bitcoin_public_key() {
    roundtrip_test::<bitcoin::PublicKey>(
        "0216867374f539badfd90d7b2269008d893ae7bd4f9ee7c695c967d01d6953c401",
    );
}

#[test]
fn ethereum_address() {
    roundtrip_test::<identity::Ethereum>("0x68917b35bacf71dbadf37628b3b7f290f6d88877");
}

#[test]
fn secrethash() {
    roundtrip_test::<SecretHash>(
        "68917b35bacf71dbadf37628b3b7f290f6d88877d7b2269008d893ae7bd4f9ee",
    );
}

#[test]
fn protocol() {
    roundtrip_test::<LockProtocol>("halbit");
    roundtrip_test::<LockProtocol>("hbit");
    roundtrip_test::<LockProtocol>("herc20");
    assert_num_variants::<LockProtocol>(3)
}

#[test]
fn position() {
    roundtrip_test::<Position>("sell");
    roundtrip_test::<Position>("buy");
    assert_num_variants::<Position>(2)
}

/// Given a string representation of a value T, this function will assert
/// that T can be constructed through the `FromStr` trait and its implementation
/// is symmetric to the `Display` implementation.
///
/// Our custom sql type `Text` relies on this behaviour being symmetric.
fn roundtrip_test<T>(stored_value: &str)
where
    <T as FromStr>::Err: fmt::Debug,
    T: fmt::Display + FromStr,
{
    // First, we verify that we can create T from the given value.
    let read = T::from_str(stored_value).unwrap();

    // Next we convert it to a string again.
    let written = read.to_string();

    // Then if we end up with the same value, our serialization is stable.
    assert_eq!(written, stored_value)
}

fn assert_num_variants<E>(expected_number_of_variants: usize)
where
    E: strum::IntoEnumIterator,
    <E as strum::IntoEnumIterator>::Iterator: Iterator,
{
    let number_of_variants = E::iter().count();

    assert_eq!(
        number_of_variants,
        expected_number_of_variants,
        "the number of variants for this enum seem to have changed, please add a serialization format test for the new variant and update the expected variant count"
    )
}
