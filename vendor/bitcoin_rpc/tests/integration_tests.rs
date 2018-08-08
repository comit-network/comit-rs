extern crate bitcoin_rpc;
extern crate coblox_bitcoincore;
extern crate jsonrpc;
#[macro_use]
extern crate log;
extern crate testcontainers;

mod common;

use bitcoin_rpc::*;
use common::{
    assert::assert_successful_result, test_client::BitcoinCoreTestClient, test_lifecycle::setup,
};
use std::collections::HashMap;

#[test]
fn test_add_multisig_address() {
    setup();

    assert_successful_result(|client| {
        let test_client = BitcoinCoreTestClient::new(client);

        let alice = test_client.an_address();
        let bob = test_client.an_address();

        client.add_multisig_address(1, vec![&alice, &bob])
    })
}

#[test]
fn test_get_block_count() {
    setup();
    assert_successful_result(BitcoinCoreClient::get_block_count)
}

#[test]
fn test_get_blockchain_info() {
    setup();
    assert_successful_result(BitcoinCoreClient::get_blockchain_info)
}

#[test]
fn test_get_new_address() {
    setup();
    assert_successful_result(BitcoinCoreClient::get_new_address)
}

#[test]
fn test_generate() {
    setup();
    assert_successful_result(|client| client.generate(1))
}

#[test]
fn test_getaccount() {
    setup();

    assert_successful_result(|client| {
        let address = BitcoinCoreTestClient::new(client).an_address();

        client.get_account(&address)
    })
}

#[test]
fn test_listunspent() {
    setup();
    assert_successful_result(|client| {
        let _ = client.generate(101);
        client.list_unspent(TxOutConfirmations::Unconfirmed, Some(101), None)
    })
}

#[test]
fn test_gettransaction() {
    setup();

    assert_successful_result(|client| {
        let tx_id = BitcoinCoreTestClient::new(client).a_transaction_id();

        client.get_transaction(&tx_id)
    })
}

#[test]
fn test_getblock() {
    setup();

    assert_successful_result(|client| {
        let block_hash = BitcoinCoreTestClient::new(client).a_block_hash();

        client.get_block(&block_hash)
    })
}

#[test]
fn test_validate_address() {
    setup();

    assert_successful_result(|client| {
        let address = BitcoinCoreTestClient::new(client).an_address();

        client.validate_address(&address)
    })
}

#[test]
fn test_get_raw_transaction_serialized() {
    setup();

    assert_successful_result(|client| {
        let tx_id = BitcoinCoreTestClient::new(client).a_transaction_id();

        client.get_raw_transaction_serialized(&tx_id)
    });
}

#[test]
fn test_decode_script() {
    setup();

    assert_successful_result(|client| {
        client.decode_script(RedeemScript::from("522103ede722780d27b05f0b1169efc90fa15a601a32fc6c3295114500c586831b6aaf2102ecd2d250a76d204011de6bc365a56033b9b3a149f679bc17205555d3c2b2854f21022d609d2f0d359e5bc0e5d0ea20ff9f5d3396cb5b1906aa9c56a0e7b5edc0c5d553ae"))
    })
}

#[test]
fn test_decode_rawtransaction() {
    setup();

    assert_successful_result(|client| {
        client.decode_rawtransaction(SerializedRawTransaction::from("0100000001bafe2175b9d7b3041ebac529056b393cf2997f7964485aa382ffa449ffdac02a000000008a473044022013d212c22f0b46bb33106d148493b9a9723adb2c3dd3a3ebe3a9c9e3b95d8cb00220461661710202fbab550f973068af45c294667fc4dc526627a7463eb23ab39e9b01410479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8ffffffff01b0a86a00000000001976a91401b81d5fa1e55e069e3cc2db9c19e2e80358f30688ac00000000"))
    })
}

#[test]
fn test_create_raw_transaction() {
    setup();

    assert_successful_result(|client| {
        let test_client = BitcoinCoreTestClient::new(client);

        let alice = test_client.an_address();
        let _ = test_client.a_block();

        let utxo = test_client.a_utxo();

        let input = NewTransactionInput::from_utxo(&utxo);
        let mut map = HashMap::new();
        map.insert(alice, utxo.amount);

        client.create_raw_transaction(vec![&input], &map)
    })
}

#[test]
fn test_dump_privkey() {
    setup();

    assert_successful_result(|client| {
        let test_client = BitcoinCoreTestClient::new(client);

        let alice = test_client.an_address();

        client.dump_privkey(&alice)
    })
}

#[test]
fn test_sign_raw_transaction() {
    setup();

    // Note: The signing actually fails but this way, we get to test the deserialization of the datastructures
    assert_successful_result(|client| {
        let test_client = BitcoinCoreTestClient::new(client);

        let alice = test_client.an_address();
        let alice_private_key = test_client
            .client
            .dump_privkey(&alice)
            .unwrap()
            .into_result()
            .unwrap();

        let utxo = test_client.a_utxo();

        let input = NewTransactionInput::from_utxo(&utxo);
        let mut map = HashMap::new();
        map.insert(alice, utxo.amount);

        let tx = test_client
            .client
            .create_raw_transaction(vec![&input], &map)
            .unwrap()
            .into_result()
            .unwrap();

        client.sign_raw_transaction(
            &tx,
            None,
            Some(vec![&alice_private_key]),
            Some(SigHashType::Single_AnyoneCanPay),
        )
    })
}

#[test]
fn test_send_to_address() {
    setup();

    assert_successful_result(|client| {
        let test_client = BitcoinCoreTestClient::new(client);
        test_client.a_block();
        let alice = test_client.an_address();

        client.send_to_address(&alice, 1.0)
    })
}

#[test]
fn test_fund_raw_transaction() {
    setup();

    assert_successful_result(|client| {
        let test_client = BitcoinCoreTestClient::new(client);

        test_client.a_block();

        let alice = test_client.an_address();

        let mut outputs = HashMap::new();
        outputs.insert(alice, 10f64);

        let raw_tx = test_client
            .client
            .create_raw_transaction(Vec::new(), &outputs)
            .unwrap()
            .into_result()
            .unwrap();
        let options = FundingOptions::new();

        client.fund_raw_transaction(&raw_tx, &options)
    })
}
