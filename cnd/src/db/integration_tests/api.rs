use crate::{
    asset,
    db::{
        wrapper_types::{Erc20Amount, EthereumAddress},
        CreatedSwap, InProgressSwap, Load, Role, Save, Sqlite,
    },
    identity,
    swap_protocols::{
        halight, herc20,
        ledger::lightning,
        rfc003::{Secret, SecretHash},
        Ledger, LocalSwapId, SharedSwapId,
    },
    timestamp::Timestamp,
};
use libp2p::{Multiaddr, PeerId};
use std::{path::PathBuf, str::FromStr};

fn temp_db() -> PathBuf {
    let temp_file = tempfile::Builder::new()
        .suffix(".sqlite")
        .tempfile()
        .unwrap();

    temp_file.into_temp_path().to_path_buf()
}

// Tests that we can save and load to the database using the database API.
#[tokio::test]
async fn roundtrip_create_finalize_load() {
    let path = temp_db();
    let db = Sqlite::new(&path).expect("a new db");

    let local_swap_id =
        LocalSwapId::from_str("111152ca-ecf2-4cc6-b35c-b4351ac28a34").expect("valid swap id");
    let role = Role::Alice;
    let peer =
        PeerId::from_str("QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY").expect("valid peer id");

    let multi_addr = "/ip4/80.123.90.4/tcp/5432";
    let address_hint: Multiaddr = multi_addr.parse().expect("valid multiaddress");

    let alpha_amount = Erc20Amount::from_str("12345").expect("valid ERC20 amount");
    let token_contract = EthereumAddress::from_str("1111e8be41b21f651a71aaB1A85c6813b8bBcCf8")
        .expect("valid etherum identity");
    let alpha_redeem_identity =
        EthereumAddress::from_str("2222e8be41b21f651a71aaB1A85c6813b8bBcCf8")
            .expect("valid redeem identity");
    let alpha_refund_identity =
        EthereumAddress::from_str("3333e8be41b21f651a71aaB1A85c6813b8bBcCf8")
            .expect("valid refund identity");
    let alpha_expiry = Timestamp::from(123u32);

    let beta_amount = asset::Bitcoin::from_sat(999);
    let beta_refund_identity = identity::Lightning::random();
    let beta_redeem_identity = identity::Lightning::random();
    let beta_expiry = Timestamp::from(456u32);

    // Simulate REST API swap POST i.e., saving a created swap.
    let created: CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap> = CreatedSwap {
        swap_id: local_swap_id,
        alpha: herc20::CreatedSwap {
            amount: alpha_amount.into(),
            identity: alpha_refund_identity.into(),
            chain_id: 1337,
            token_contract: token_contract.into(),
            absolute_expiry: alpha_expiry.into(),
        },
        beta: halight::CreatedSwap {
            amount: beta_amount,
            identity: beta_redeem_identity,
            network: lightning::Regtest,
            cltv_expiry: beta_expiry.into(),
        },
        peer,
        address_hint: Some(address_hint),
        role,
    };
    Save::<CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>>::save(&db, created)
        .await
        .expect("to be able to save created swap");

    // Simulate announce message.
    let shared_swap_id =
        SharedSwapId::from_str("222252ca-ecf2-4cc6-b35c-b4351ac28a34").expect("valid swap id");
    db.save_shared_swap_id(local_swap_id, shared_swap_id)
        .await
        .expect("to be able to save shared swap id");

    // Simulate secret_hash message.
    let secret = Secret::from(*b"This is our favourite passphrase");
    let secret_hash = SecretHash::from(secret);

    db.save_secret_hash(local_swap_id, secret_hash)
        .await
        .expect("to be able to save secret hash");

    // Simulate identity messages.
    db.save_counterparty_halight_refund_identity(local_swap_id, beta_refund_identity)
        .await
        .expect("to be able to save Lightning refund identity");
    db.save_counterparty_herc20_redeem_identity(local_swap_id, alpha_redeem_identity.into())
        .await
        .expect("to be able to save Ethereum redeem identity");

    let want: InProgressSwap<herc20::InProgressSwap, halight::InProgressSwap> = InProgressSwap {
        swap_id: local_swap_id,
        secret_hash,
        role,
        alpha: herc20::InProgressSwap {
            ledger: Ledger::Alpha,
            refund_identity: alpha_refund_identity.into(),
            redeem_identity: alpha_redeem_identity.into(),
            expiry: alpha_expiry,
        },
        beta: halight::InProgressSwap {
            ledger: Ledger::Beta,
            asset: beta_amount,
            refund_identity: beta_refund_identity,
            redeem_identity: beta_redeem_identity,
            expiry: beta_expiry,
        },
    };

    let got = Load::<InProgressSwap<herc20::InProgressSwap, halight::InProgressSwap>>::load(
        &db,
        local_swap_id,
    )
    .await
    .expect("to be able to load an in progress swap")
    .expect("to have gotten some in progress swap");

    assert_eq!(got, want);
}
