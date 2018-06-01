extern crate env_logger;
extern crate ethereum_htlc;
extern crate ganache_rust_web3;
extern crate hex;
extern crate web3;
#[macro_use]
extern crate log;

use web3::types::Bytes;
use web3::types::U256;

mod common;
use common::GanacheClient;

#[test]
fn given_deployed_htlc_when_redeemed_with_secret_then_money_is_transferred() {
    let _ = env_logger::try_init();

    let refund_address = "5C5472FeFf4c7526C1C89A9f29229C007c88Df72".into_address();
    let success_address = "73782035b894Ed39985fbF4062e695b8e524Ca4E".into_address();

    const SECRET: &[u8] = b"hello world, you are beautiful!!";
    let secret_hash =
        "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec".into_secret_hash();

    let htlc = eth_htlc::Htlc::new(
        EpochOffset::hours(12),
        refund_address,
        success_address,
        secret_hash,
    );

    let mut client = GanacheClient::new();

    client.take_snapshot();

    let contract_address = client.deploy(refund_address, htlc, 10);

    let refund_balance_before_htlc = client.get_balance(refund_address);
    let success_balance_before_htlc = client.get_balance(success_address);

    let gas_used = client.send_data(
        refund_address,
        contract_address,
        Some(Bytes(SECRET.to_vec())),
    );

    let refund_balance_after_htlc = client.get_balance(refund_address);
    let success_balance_after_htlc = client.get_balance(success_address);

    client.restore_snapshot();

    assert_eq!(
        success_balance_after_htlc.checked_sub(success_balance_before_htlc),
        Some(U256::from(10))
    );
    assert_eq!(
        refund_balance_before_htlc - gas_used,
        refund_balance_after_htlc
    );
}

#[test]
fn given_deployed_htlc_when_refunded_after_timeout_then_money_is_refunded() {
    let _ = env_logger::try_init();

    let refund_address = "c32bec6b4d0457a7cb3974ed23c6837d054ce0b2".into_address();
    let success_address = "2d59c93d4664ea878c2d862b7896caf2efbd67a6".into_address();

    let secret_hash =
        "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec".into_secret_hash();

    let htlc = eth_htlc::Htlc::new(
        EpochOffset::hours(1),
        refund_address,
        success_address,
        secret_hash,
    );

    let mut client = GanacheClient::new();

    client.take_snapshot();

    let contract_address = client.deploy(refund_address, htlc, 10);

    let refund_balance_before_htlc = client.get_balance(refund_address);
    let success_balance_before_htlc = client.get_balance(success_address);

    client.activate_flux_compensator(2);

    let gas_used = client.send_data(refund_address, contract_address, None);

    let refund_balance_after_htlc = client.get_balance(refund_address);
    let success_balance_after_htlc = client.get_balance(success_address);

    client.restore_snapshot();

    assert_eq!(success_balance_after_htlc, success_balance_before_htlc);
    assert_eq!(
        refund_balance_before_htlc - gas_used + U256::from(10),
        refund_balance_after_htlc
    );
}

#[test]
fn given_deployed_htlc_when_timeout_not_yet_reached_and_wrong_secret_then_nothing_happens() {
    let _ = env_logger::try_init();

    let refund_address = "97057571fb0cb8420aff01164ac342e3525ee274".into_address();
    let success_address = "f1fd72baa06a9806b75b7302460510586d6f54e8".into_address();

    let secret_hash =
        "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec".into_secret_hash();

    // FIXME Unfortunately, evm_increaseTime cannot be undone at the moment, so we have to add 2 hours for the increase of the last test.
    // As soon as a version of ganache-cli with ganache-core > v2.1.0 is released (https://github.com/trufflesuite/ganache-core/releases),
    // we can remove this because then https://github.com/trufflesuite/ganache-core/pull/2 is included in the release.
    let stupid_offset = 2;

    let htlc = eth_htlc::Htlc::new(
        EpochOffset::hours(1 + stupid_offset),
        refund_address,
        success_address,
        secret_hash,
    );

    let mut client = GanacheClient::new();

    client.take_snapshot();

    let contract_address = client.deploy(refund_address, htlc, 10);

    let refund_balance_before_htlc = client.get_balance(refund_address);
    let success_balance_before_htlc = client.get_balance(success_address);

    let gas_used = client.send_data(refund_address, contract_address, None);

    let refund_balance_after_htlc = client.get_balance(refund_address);
    let success_balance_after_htlc = client.get_balance(success_address);

    client.restore_snapshot();

    assert_eq!(success_balance_after_htlc, success_balance_before_htlc);
    assert_eq!(
        refund_balance_before_htlc - gas_used,
        refund_balance_after_htlc
    );
}
