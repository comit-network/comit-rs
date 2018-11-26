extern crate comit_node;
extern crate ethereum_support;
extern crate hex;
extern crate pretty_env_logger;
extern crate rand;
extern crate rlp;
extern crate secp256k1_support;
extern crate tc_web3_client;
extern crate testcontainers;
extern crate tiny_keccak;

mod ethereum_wallet;

use ethereum_support::*;
use ethereum_wallet::*;
use secp256k1_support::KeyPair;
use testcontainers::{clients::Cli, images::parity_parity::ParityEthereum, Docker};

#[test]
fn given_manually_signed_transaction_when_sent_then_it_spends_from_correct_address() {
    let _ = pretty_env_logger::try_init();

    // Arrange
    let keypair = KeyPair::new(&mut rand::thread_rng());
    let parity_dev_account = "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap();
    let wallet = InMemoryWallet::new(keypair, 0x11); // 0x11 is the chain id of the parity dev chain

    let account = keypair.public_key().to_ethereum_address();
    let docker = Cli::default();

    let container = docker.run(ParityEthereum::default());
    let (_event_loop, client) = tc_web3_client::new(&container);

    client
        .personal()
        .send_transaction(
            TransactionRequest {
                from: parity_dev_account,
                to: Some(account),
                gas: None,
                gas_price: None,
                value: Some(U256::from(100_000_000_000_000_000u64)),
                data: None,
                nonce: None,
                condition: None,
            },
            "",
        )
        .wait()
        .unwrap();

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

    let value = U256::from(100_000);

    let tx: UnsignedTransaction = ethereum_wallet::UnsignedTransaction::new_payment(
        "73782035b894ed39985fbf4062e695b8e524ca4e",
        1,
        value,
        get_nonce(),
        None,
    );

    let tx = wallet.sign(&tx);

    let balance_before_tx = get_balance();

    // Act

    let gas_used = send_transaction(tx.into());

    // Assert

    let balance_after_tx = get_balance();

    assert_eq!(balance_before_tx - value - gas_used, balance_after_tx);
}
