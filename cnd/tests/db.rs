use bitcoin::{self, secp256k1::Secp256k1};
use chrono::NaiveDateTime;
use cnd::{
    bitcoin::PublicKey,
    db::{AcceptedSwap, LoadAcceptedSwap, Save, Sqlite, Swap},
    ethereum::{Address, EtherQuantity},
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{Accept, Request, Secret, SecretHash},
        HashFunction, Role, SwapId,
    },
    timestamp::Timestamp,
};
use futures_core::{FutureExt, TryFutureExt};
use libp2p::PeerId;
use std::str::FromStr;
use tokio::{self, runtime::current_thread::Runtime};

#[test]
fn accepted_swap_has_valid_timestamp() {
    let mut runtime = Runtime::new().expect("failed to create new runtime");

    let future = accepted_at_for_bitcoin_ethereum();
    // If this assignment works then we have a valid NaiveDateTime.
    let _accepted_at = runtime
        .block_on(future.boxed().compat())
        .expect("failed to get accepted swap");

    let future = accepted_at_for_ethereum_bitcoin();
    let _accepted_at = runtime
        .block_on(future.boxed().compat())
        .expect("failed to get accepted swap");
}

async fn accepted_at_for_bitcoin_ethereum() -> anyhow::Result<NaiveDateTime> {
    let swap_id = swap_id();
    let role = Role::Alice;

    let db_path = tempfile::Builder::new()
        .prefix(&swap_id.to_string())
        .suffix(".sqlite")
        .tempfile()
        .unwrap()
        .into_temp_path();
    let db = Sqlite::new(&db_path).expect("db");

    let swap = swap(swap_id, role);
    db.save(swap).await?;

    let request = Request {
        swap_id,
        alpha_ledger: Bitcoin::default(),
        beta_ledger: Ethereum::default(),
        alpha_asset: bitcoin::Amount::default(),
        beta_asset: EtherQuantity::zero(),
        hash_function: HashFunction::Sha256,
        alpha_ledger_refund_identity: bitcoin_address(),
        beta_ledger_redeem_identity: ethereum_address(),
        alpha_expiry: Timestamp::now(),
        beta_expiry: Timestamp::now(),
        secret_hash: secret_hash(),
    };
    db.save(request).await?;

    let accept: Accept<Bitcoin, Ethereum> = Accept {
        swap_id,
        beta_ledger_refund_identity: ethereum_address(), // This is non-sense but fine for this test
        alpha_ledger_redeem_identity: bitcoin_address(), // same address for refund/redeem.
    };
    db.save(accept).await?;

    let accepted_swap: AcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, EtherQuantity> =
        db.load_accepted_swap(&swap_id).await?;

    let (_request, _accept, at) = accepted_swap;

    Ok(at)
}

async fn accepted_at_for_ethereum_bitcoin() -> anyhow::Result<NaiveDateTime> {
    let swap_id = swap_id();
    let role = Role::Bob;

    let db_path = tempfile::Builder::new()
        .prefix(&swap_id.to_string())
        .suffix(".sqlite")
        .tempfile()
        .unwrap()
        .into_temp_path();
    let db = Sqlite::new(&db_path).expect("db");

    let swap = swap(swap_id, role);
    db.save(swap).await?;

    let request = Request {
        swap_id,
        alpha_ledger: Ethereum::default(),
        beta_ledger: Bitcoin::default(),
        alpha_asset: EtherQuantity::zero(),
        beta_asset: bitcoin::Amount::default(),
        hash_function: HashFunction::Sha256,
        alpha_ledger_refund_identity: ethereum_address(),
        beta_ledger_redeem_identity: bitcoin_address(),
        alpha_expiry: Timestamp::now(),
        beta_expiry: Timestamp::now(),
        secret_hash: secret_hash(),
    };
    db.save(request).await?;

    let accept: Accept<Ethereum, Bitcoin> = Accept {
        swap_id,
        beta_ledger_refund_identity: bitcoin_address(), // This is non-sense but fine for this test
        alpha_ledger_redeem_identity: ethereum_address(), // same address for refund/redeem.
    };
    db.save(accept).await?;

    let accepted_swap: AcceptedSwap<Ethereum, Bitcoin, EtherQuantity, bitcoin::Amount> =
        db.load_accepted_swap(&swap_id).await?;

    let (_request, _accept, at) = accepted_swap;

    Ok(at)
}

fn swap_id() -> SwapId {
    SwapId::from_str("0123456789abcdef0123456789abcdef").unwrap()
}

fn swap(swap_id: SwapId, role: Role) -> Swap {
    Swap {
        swap_id,
        role,
        counterparty: PeerId::random(),
    }
}

fn bitcoin_address() -> PublicKey {
    let s = Secp256k1::new();
    let sk = bitcoin::PrivateKey::from_str("cVt4o7BGAig1UXywgGSmARhxMdzP5qvQsxKkSsc1XEkw3tDTQFpy")
        .unwrap();
    let pk = bitcoin::PublicKey::from_private_key(&s, &sk);

    PublicKey::from(pk)
}

fn ethereum_address() -> Address {
    Address::from_str("0A81e8be41b21f651a71aaB1A85c6813b8bBcCf8").unwrap()
}

fn secret_hash() -> SecretHash {
    let bytes = b"hello world, you are beautiful!!";
    let secret = Secret::from(*bytes);
    secret.hash()
}
