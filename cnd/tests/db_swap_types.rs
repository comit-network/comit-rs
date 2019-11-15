use bitcoin::{self, secp256k1::Secp256k1};
use cnd::{
    bitcoin::PublicKey,
    db::{AssetKind, DetermineTypes, LedgerKind, Save, SaveMessage, Sqlite, Swap, SwapTypes},
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{Request, Secret, SecretHash},
        HashFunction, Role, SwapId, Timestamp,
    },
};
use ethereum_support::{Address, Erc20Quantity, Erc20Token, EtherQuantity};
use libp2p::PeerId;
use std::str::FromStr;

fn swap_id() -> SwapId {
    SwapId::from_str("0123456789abcdef0123456789abcdef").unwrap()
}

fn bitcoin_address() -> PublicKey {
    static KEY_WIF: &'static str = "cVt4o7BGAig1UXywgGSmARhxMdzP5qvQsxKkSsc1XEkw3tDTQFpy";
    let s = Secp256k1::new();
    let sk = bitcoin::PrivateKey::from_str(&KEY_WIF).unwrap();
    let pk = bitcoin::PublicKey::from_private_key(&s, &sk);

    PublicKey::from(pk)
}

fn ethereum_address() -> Address {
    Address::from_str("0A81e8be41b21f651a71aaB1A85c6813b8bBcCf8").unwrap()
}

fn erc20_token() -> Erc20Token {
    Erc20Token {
        token_contract: ethereum_address(),
        quantity: Erc20Quantity::zero(),
    }
}

fn secret_hash() -> SecretHash {
    let bytes = b"hello world, you are beautiful!!";
    let secret = Secret::from(*bytes);
    secret.hash()
}

#[test]
fn can_determine_correct_swap_types() {
    can_determine_bitcoin_ethereum_bitcoin_ether();
    can_determine_bitcoin_ethereum_bitcoin_erc20();
    can_determine_ethereum_bitcoin_ether_bitcoin();
    can_determine_ethereum_bitcoin_erc20_bitcoin();
}

#[test]
fn can_determine_bitcoin_ethereum_bitcoin_ether() {
    let swap_id = swap_id();
    let role = Role::Alice;

    let db_path = tempfile::Builder::new()
        .prefix(&swap_id.to_string())
        .suffix(".sqlite")
        .tempfile()
        .unwrap()
        .into_temp_path();
    let db = Sqlite::new(&db_path).expect("db");

    let swap = swap(swap_id.clone(), role);
    db.save(swap.clone()).expect("save");

    let request = Request {
        swap_id: swap_id.clone(),
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
    db.save_message(request).expect("save message");

    let want_types = swap_types_bitcoin_ethereum_bitcoin_ether(role);
    let got_types = db.determine_types(&swap_id).expect("determine types");

    assert_eq!(want_types, got_types);
}

#[test]
fn can_determine_bitcoin_ethereum_bitcoin_erc20() {
    let swap_id = swap_id();
    let role = Role::Alice;

    let db_path = tempfile::Builder::new()
        .prefix(&swap_id.to_string())
        .suffix(".sqlite")
        .tempfile()
        .unwrap()
        .into_temp_path();
    let db = Sqlite::new(&db_path).expect("db");

    let swap = swap(swap_id.clone(), role);
    db.save(swap.clone()).expect("save");

    let request = Request {
        swap_id: swap_id.clone(),
        alpha_ledger: Bitcoin::default(),
        beta_ledger: Ethereum::default(),
        alpha_asset: bitcoin::Amount::default(),
        beta_asset: erc20_token(),
        hash_function: HashFunction::Sha256,
        alpha_ledger_refund_identity: bitcoin_address(),
        beta_ledger_redeem_identity: ethereum_address(),
        alpha_expiry: Timestamp::now(),
        beta_expiry: Timestamp::now(),
        secret_hash: secret_hash(),
    };
    db.save_message(request).expect("save message");

    let want_types = swap_types_bitcoin_ethereum_bitcoin_erc20(role);
    let got_types = db.determine_types(&swap_id).expect("determine types");

    assert_eq!(want_types, got_types);
}

#[test]
fn can_determine_ethereum_bitcoin_ether_bitcoin() {
    let swap_id = swap_id();
    let role = Role::Bob;

    let db_path = tempfile::Builder::new()
        .prefix(&swap_id.to_string())
        .suffix(".sqlite")
        .tempfile()
        .unwrap()
        .into_temp_path();
    let db = Sqlite::new(&db_path).expect("db");

    let swap = swap(swap_id.clone(), role);
    db.save(swap.clone()).expect("save");

    let request = Request {
        swap_id: swap_id.clone(),
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
    db.save_message(request).expect("save message");

    let want_types = swap_types_ethereum_bitcoin_ether_bitcoin(role);
    let got_types = db.determine_types(&swap_id).expect("determine types");

    assert_eq!(want_types, got_types);
}

#[test]
fn can_determine_ethereum_bitcoin_erc20_bitcoin() {
    let swap_id = swap_id();
    let role = Role::Bob;

    let db_path = tempfile::Builder::new()
        .prefix(&swap_id.to_string())
        .suffix(".sqlite")
        .tempfile()
        .unwrap()
        .into_temp_path();
    let db = Sqlite::new(&db_path).expect("db");

    let swap = swap(swap_id.clone(), role);
    db.save(swap.clone()).expect("save");

    let request = Request {
        swap_id: swap_id.clone(),
        alpha_ledger: Ethereum::default(),
        beta_ledger: Bitcoin::default(),
        alpha_asset: erc20_token(),
        beta_asset: bitcoin::Amount::default(),
        hash_function: HashFunction::Sha256,
        alpha_ledger_refund_identity: ethereum_address(),
        beta_ledger_redeem_identity: bitcoin_address(),
        alpha_expiry: Timestamp::now(),
        beta_expiry: Timestamp::now(),
        secret_hash: secret_hash(),
    };
    db.save_message(request).expect("save message");

    let want_types = swap_types_ethereum_bitcoin_erc20_bitcoin(role);
    let got_types = db.determine_types(&swap_id).expect("determine types");

    assert_eq!(want_types, got_types);
}

fn swap(swap_id: SwapId, role: Role) -> Swap {
    Swap {
        swap_id,
        role,
        counterparty: PeerId::random(),
    }
}

fn swap_types_bitcoin_ethereum_bitcoin_ether(role: Role) -> SwapTypes {
    SwapTypes {
        alpha_ledger: LedgerKind::Bitcoin,
        beta_ledger: LedgerKind::Ethereum,
        alpha_asset: AssetKind::Bitcoin,
        beta_asset: AssetKind::Ether,
        role,
    }
}

fn swap_types_bitcoin_ethereum_bitcoin_erc20(role: Role) -> SwapTypes {
    SwapTypes {
        alpha_ledger: LedgerKind::Bitcoin,
        beta_ledger: LedgerKind::Ethereum,
        alpha_asset: AssetKind::Bitcoin,
        beta_asset: AssetKind::Erc20,
        role,
    }
}

fn swap_types_ethereum_bitcoin_ether_bitcoin(role: Role) -> SwapTypes {
    SwapTypes {
        alpha_ledger: LedgerKind::Ethereum,
        beta_ledger: LedgerKind::Bitcoin,
        alpha_asset: AssetKind::Ether,
        beta_asset: AssetKind::Bitcoin,
        role,
    }
}

fn swap_types_ethereum_bitcoin_erc20_bitcoin(role: Role) -> SwapTypes {
    SwapTypes {
        alpha_ledger: LedgerKind::Ethereum,
        beta_ledger: LedgerKind::Bitcoin,
        alpha_asset: AssetKind::Erc20,
        beta_asset: AssetKind::Bitcoin,
        role,
    }
}
