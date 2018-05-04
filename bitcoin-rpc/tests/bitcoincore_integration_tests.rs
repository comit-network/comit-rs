extern crate bitcoin_rpc;
extern crate env_logger;
extern crate jsonrpc;
#[macro_use]
extern crate log;

use bitcoin_rpc::*;
use jsonrpc::{HTTPError, RpcError, RpcResponse};

use std::fmt::Debug;

#[test]
fn test_add_multisig_address() {
    let _ = env_logger::try_init();
    assert_successful_result(|client| {
        let address = client.get_new_address().unwrap().into_result().unwrap();

        client.add_multisig_address(1, vec![address])
    })
}

#[test]
fn test_get_block_count() {
    let _ = env_logger::try_init();
    assert_successful_result(BitcoinCoreClient::get_block_count)
}

#[test]
fn test_get_new_address() {
    let _ = env_logger::try_init();
    assert_successful_result(BitcoinCoreClient::get_new_address)
}

#[test]
fn test_generate() {
    let _ = env_logger::try_init();
    assert_successful_result(|client| client.generate(1))
}

#[test]
fn test_getaccount() {
    let _ = env_logger::try_init();
    assert_successful_result(|client| {
        client.get_account(Address::from("2N2PMtfaKc9knQYxmTZRg3dcC1SMZ7SC8PW"))
    })
}

#[test]
fn test_gettransaction() {
    let _ = env_logger::try_init();
    assert_successful_result(|client| {
        client.get_transaction(TransactionId::from(
            "70935ecf77405bccda14ed73a7e2d79f0bb75e0b1c06b8f1c3c2e3f6b600ff46",
        ))
    })
}

#[test]
fn test_getblock() {
    let _ = env_logger::try_init();
    let generated_blocks = new_bitcoin_client()
        .generate(1)
        .unwrap()
        .into_result()
        .unwrap();
    let block_hash = generated_blocks.get(0).unwrap().to_owned();

    assert_successful_result(|client| client.get_block(block_hash))
}

#[test]
fn test_validate_address() {
    let _ = env_logger::try_init();
    let address = new_bitcoin_client()
        .get_new_address()
        .unwrap()
        .into_result()
        .unwrap();

    assert_successful_result(|client| client.validate_address(&address))
}

#[test]
fn test_get_raw_transaction_serialized() {
    let _ = env_logger::try_init();
    let client = new_bitcoin_client();

    let block = client
        .generate(1)
        .and_then(|response| {
            let blocks = response.into_result().unwrap();
            let block = blocks.get(0).unwrap();
            client.get_block(block)
        })
        .unwrap()
        .into_result()
        .unwrap();

    let tx_id = block.tx.get(0).unwrap();

    assert_successful_result(|client| client.get_raw_transaction_serialized(tx_id));
}

#[test]
fn test_decode_script() {
    let _ = env_logger::try_init();

    assert_successful_result(|client| {
        client.decode_script(RedeemScript::from("522103ede722780d27b05f0b1169efc90fa15a601a32fc6c3295114500c586831b6aaf2102ecd2d250a76d204011de6bc365a56033b9b3a149f679bc17205555d3c2b2854f21022d609d2f0d359e5bc0e5d0ea20ff9f5d3396cb5b1906aa9c56a0e7b5edc0c5d553ae"))
    })
}

#[test]
fn test_decode_rawtransaction() {
    let _ = env_logger::try_init();

    assert_successful_result(|client| {
        client.decode_rawtransaction(SerializedRawTransaction::from("0100000001bafe2175b9d7b3041ebac529056b393cf2997f7964485aa382ffa449ffdac02a000000008a473044022013d212c22f0b46bb33106d148493b9a9723adb2c3dd3a3ebe3a9c9e3b95d8cb00220461661710202fbab550f973068af45c294667fc4dc526627a7463eb23ab39e9b01410479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8ffffffff01b0a86a00000000001976a91401b81d5fa1e55e069e3cc2db9c19e2e80358f30688ac00000000"))
    })
}

fn new_bitcoin_client() -> BitcoinCoreClient {
    let url = env!("BITCOIN_RPC_URL");
    let username = env!("BITCOIN_RPC_USERNAME");
    let password = env!("BITCOIN_RPC_PASSWORD");

    BitcoinCoreClient::new(url, username, password)
}

fn assert_successful_result<R, I>(invocation: I)
where
    R: Debug,
    I: Fn(&BitcoinCoreClient) -> Result<RpcResponse<R>, HTTPError>,
{
    let client = new_bitcoin_client();
    let result: Result<R, RpcError> = invocation(&client).unwrap().into();

    if result.is_err() {
        error!("{:?}", result.unwrap_err());
        panic!("Result should be successful")
    } else {
        // Having a successful result means:
        // - No HTTP Error occured
        // - No deserialization error occured
        debug!("{:?}", result.unwrap())
    }
}
