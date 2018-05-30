extern crate eth_htlc;
extern crate hex;
extern crate web3;

use eth_htlc::{EpochOffset, IntoAddress, IntoSecretHash};
use std::env::var;
use web3::futures::Future;
use web3::types::Bytes;
use web3::types::TransactionRequest;
use web3::types::U256;

#[test]
fn given_deployed_htlc_when_redeemed_with_secret_then_money_is_transferred() {
    let refund_address = "5C5472FeFf4c7526C1C89A9f29229C007c88Df72".into_address();
    let success_address = "73782035b894Ed39985fbF4062e695b8e524Ca4E".into_address();

    const SECRET: &[u8] = b"hello world, you are beautiful!!";
    let secret_hash =
        "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec".into_secret_hash();

    let endpoint = var("GANACHE_ENDPOINT").unwrap();

    let (_eloop, transport) = web3::transports::Http::new(&endpoint).unwrap();
    let web3 = web3::Web3::new(transport);

    let htlc = eth_htlc::Htlc::new(
        EpochOffset::hours(12),
        refund_address,
        success_address,
        secret_hash,
    );

    let compiled_contract = htlc.compile_to_hex();

    let htlc_value = 10;

    let contract_tx_id = web3.eth()
        .send_transaction(TransactionRequest {
            from: refund_address,
            to: None,
            gas: None,
            gas_price: None,
            value: Some(U256::from(htlc_value)),
            data: Some(compiled_contract.into_bytes()),
            nonce: None,
            condition: None,
        })
        .wait()
        .unwrap();

    let receipt = web3.eth()
        .transaction_receipt(contract_tx_id)
        .wait()
        .unwrap()
        .unwrap();

    let contract_address = receipt.contract_address.unwrap();

    let refund_balance_before_htlc = web3.eth().balance(refund_address, None).wait().unwrap();
    let success_balance_before_htlc = web3.eth().balance(success_address, None).wait().unwrap();

    let result_tx = web3.eth()
        .send_transaction(TransactionRequest {
            from: refund_address,
            to: Some(contract_address),
            gas: None,
            gas_price: None,
            value: None,
            data: Some(Bytes(SECRET.to_vec())),
            nonce: None,
            condition: None,
        })
        .wait()
        .unwrap();

    let receipt = web3.eth()
        .transaction_receipt(result_tx)
        .wait()
        .unwrap()
        .unwrap();

    let refund_balance_after_htlc = web3.eth().balance(refund_address, None).wait().unwrap();
    let success_balance_after_htlc = web3.eth().balance(success_address, None).wait().unwrap();

    assert_eq!(
        success_balance_after_htlc.checked_sub(success_balance_before_htlc),
        Some(U256::from(htlc_value))
    );
    assert_eq!(
        refund_balance_before_htlc - receipt.gas_used,
        refund_balance_after_htlc
    );
}

#[test]
fn given_deployed_htlc_when_refunded_after_timeout_then_money_is_refunded() {}

fn given_deployed_htlc_when_timeout_not_yet_reached_and_wrong_secret_then_nothing_happens() {}
