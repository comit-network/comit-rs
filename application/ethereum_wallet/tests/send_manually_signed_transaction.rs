extern crate env_logger;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate hex;
extern crate secp256k1_support;
extern crate tc_web3_client;
extern crate testcontainers;

use ethereum_support::*;
use ethereum_wallet::*;
use hex::FromHex;
use secp256k1_support::KeyPair;
use testcontainers::{clients::Cli, images::trufflesuite_ganachecli::GanacheCli, Docker};

#[test]
fn given_manually_signed_transaction_when_sent_then_it_spends_from_correct_address() {
    let _ = env_logger::try_init();

    // Arrange

    let account = Address::from("e7b6bfabddfaeb2c016b334a5322e4327dc5e499");
    let docker = Cli::default();

    let container = docker.run(GanacheCli::default());
    let (_event_loop, client) = tc_web3_client::new(&container);

    let get_nonce = || {
        client
            .eth()
            .transaction_count(account, None)
            .wait()
            .unwrap()
    };
    let get_balance = || client.eth().balance(account, None).wait().unwrap();
    let send_transaction = |transaction| {
        let txid = client
            .eth()
            .send_raw_transaction(transaction)
            .wait()
            .unwrap();
        let receipt = client
            .eth()
            .transaction_receipt(txid)
            .wait()
            .unwrap()
            .unwrap();

        receipt.gas_used
    };

    let wallet = {
        let secret_key_data = &<[u8; 32]>::from_hex(
            "a710faa76db883cd246112142b609bfe2f122b362b85719f47d91541e104b33d",
        ).unwrap();
        let keypair = KeyPair::from_secret_key_slice(secret_key_data).unwrap();
        InMemoryWallet::new(keypair, 42) // 42 is used in GanacheCliNode
    };

    let tx = UnsignedTransaction::new_payment(
        "73782035b894ed39985fbf4062e695b8e524ca4e",
        1,
        0,
        get_nonce(),
        None,
    );

    let tx = wallet.sign(&tx);

    let balance_before_tx = get_balance();

    // Act

    let gas_used = send_transaction(tx.into());

    // Assert

    let balance_after_tx = get_balance();

    assert_eq!(balance_before_tx - gas_used, balance_after_tx);
}
