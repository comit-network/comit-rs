extern crate bitcoin_rpc;
extern crate jsonrpc;
#[macro_use]
extern crate log;

mod common;

use bitcoin_rpc::*;
use common::test_client::BitcoinCoreTestClient;
use common::assert::assert_successful_result;
use common::test_lifecycle::setup;

#[test]
fn test_add_multisig_address() {
    setup();

    let alice = BitcoinCoreTestClient::new().an_address();
    let bob = BitcoinCoreTestClient::new().an_address();

    assert_successful_result(|client| client.add_multisig_address(1, vec![&alice, &bob]))
}

#[test]
fn test_get_block_count() {
    setup();
    assert_successful_result(BitcoinCoreClient::get_block_count)
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

    let address = BitcoinCoreTestClient::new().an_address();

    assert_successful_result(|client| client.get_account(&address))
}

//#[test]
//fn test_listunspent() {
//    setup();
//    assert_successful_result(|client| {
//        client.generate(1);
//        client.list_unspent(TxOutConfirmations::AtLeast(5), None, None)
//    })
//}

#[test]
fn test_gettransaction() {
    setup();

    let tx_id = BitcoinCoreTestClient::new().a_transaction_id();

    assert_successful_result(|client| client.get_transaction(&tx_id))
}

#[test]
fn test_getblock() {
    setup();

    let block_hash = BitcoinCoreTestClient::new().a_block_hash();

    assert_successful_result(|client| client.get_block(&block_hash))
}

#[test]
fn test_validate_address() {
    setup();

    let address = BitcoinCoreTestClient::new().an_address();

    assert_successful_result(|client| client.validate_address(&address))
}

#[test]
fn test_get_raw_transaction_serialized() {
    setup();

    let tx_id = BitcoinCoreTestClient::new().a_transaction_id();

    assert_successful_result(|client| client.get_raw_transaction_serialized(&tx_id));
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
