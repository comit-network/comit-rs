extern crate ethereum_wallet;
extern crate hex;
extern crate web3;

use ethereum_wallet::*;
use hex::FromHex;
use std::env::var;
use web3::futures::Future;

#[test]
fn given_manually_signed_transaction_when_sent_then_it_spends_from_correct_address() {
    // Arrange

    let account: web3::types::Address = "e7b6bfabddfaeb2c016b334a5322e4327dc5e499".into();

    let endpoint = var("GANACHE_ENDPOINT").unwrap_or("http://localhost:7545".to_string());
    let (_event_loop, transport) = web3::transports::Http::new(&endpoint).unwrap();
    let web3 = web3::api::Web3::new(transport);

    let get_nonce = || web3.eth().transaction_count(account, None).wait().unwrap();
    let get_balance = || web3.eth().balance(account, None).wait().unwrap();
    let send_transaction = |bytes| {
        let txid = web3.eth().send_raw_transaction(bytes).wait().unwrap();
        let receipt = web3.eth()
            .transaction_receipt(txid)
            .wait()
            .unwrap()
            .unwrap();

        receipt.gas_used
    };

    let wallet = {
        let private_key = <[u8; 32]>::from_hex(
            "a710faa76db883cd246112142b609bfe2f122b362b85719f47d91541e104b33d",
        ).unwrap();
        InMemoryWallet::new(private_key, 1).unwrap()
    };

    let tx = Transaction::new_payment(
        "73782035b894ed39985fbf4062e695b8e524ca4e",
        1000000,
        1,
        0,
        get_nonce(),
    );

    let bytes = wallet.create_signed_raw_transaction(&tx);

    let balance_before_tx = get_balance();

    // Act

    let gas_used = send_transaction(bytes);

    // Assert

    let balance_after_tx = get_balance();

    assert_eq!(balance_before_tx - gas_used, balance_after_tx);
}
