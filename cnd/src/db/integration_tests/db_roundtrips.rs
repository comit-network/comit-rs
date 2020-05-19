use crate::{
    asset::{Bitcoin as BitcoinAsset, Erc20, Ether},
    db::{
        load_swaps::LoadAcceptedSwap,
        swap_types::{DetermineTypes, SwapTypes},
        AssetKind, LedgerKind, Retrieve, Save, Sqlite, Swap,
    },
    identity,
    quickcheck::Quickcheck,
    swap_protocols::{
        ledger::{self, Ethereum},
        rfc003::{Accept, Request},
    },
};

macro_rules! db_roundtrip_test {
    ($alpha_ledger:ident, $beta_ledger:ident, $alpha_asset:ident, $beta_asset:ident, $alpha_identity:ident, $beta_identity:ident, $expected_swap_types_fn:expr) => {
        paste::item! {
            #[test]
            #[allow(non_snake_case, clippy::redundant_closure_call)]
            fn [<roundtrip_test_ $alpha_ledger _ $beta_ledger _ $alpha_asset _ $beta_asset>]() {
                fn prop(swap: Quickcheck<Swap>,
                        request: Quickcheck<Request<$alpha_ledger, $beta_ledger, $alpha_asset, $beta_asset, $alpha_identity, $beta_identity>>,
                        accept: Quickcheck<Accept<$alpha_identity, $beta_identity>>,
                ) -> anyhow::Result<bool> {

                    // unpack the swap from the generic newtype
                    let Swap { swap_id, role, counterparty } = swap.0;

                    // construct the expected swap types from the function we get passed in order to enrich it with the role
                    let expected_swap_types = ($expected_swap_types_fn)(role);

                    let db = Sqlite::test();

                    let saved_swap = Swap {
                        swap_id,
                        role,
                        counterparty
                    };
                    let saved_request = Request {
                        swap_id,
                        ..(*request).clone()
                    };
                    let saved_accept = Accept {
                        swap_id,
                        ..*accept
                    };

                    let (loaded_swap, loaded_request, loaded_accept, loaded_swap_types) =
                    tokio::runtime::Runtime::new()?.block_on(async {
                        db.save(saved_swap.clone()).await?;
                        db.save(saved_request.clone()).await?;
                        db.save(saved_accept.clone()).await?;

                        let loaded_swap = Retrieve::get(&db, &swap_id).await?;
                        // If the assignment of `_at` works then we have a valid NaiveDateTime.
                        let (loaded_request, loaded_accept, _at) = db.load_accepted_swap(&swap_id).await?;
                        let loaded_swap_types = db.determine_types(&swap_id).await?;

                        anyhow::Result::<_>::Ok((loaded_swap, loaded_request, loaded_accept, loaded_swap_types))
                    })?;

                    Ok(
                        saved_request == loaded_request &&
                            saved_accept == loaded_accept &&
                            saved_swap == loaded_swap &&
                            expected_swap_types == loaded_swap_types
                    )
                }

                quickcheck::quickcheck(prop as fn(
                    Quickcheck<Swap>,
                    Quickcheck<Request<$alpha_ledger, $beta_ledger, $alpha_asset, $beta_asset, $alpha_identity, $beta_identity>>,
                    Quickcheck<Accept<$alpha_identity, $beta_identity>>,
                ) -> anyhow::Result<bool>);
            }
        }
    };
}

// do_roundtrip_test! does not seem to like being called with `::` in an ident.
use identity::{Bitcoin as BitcoinIdentity, Ethereum as EthereumIdentity};
use ledger::Bitcoin as BitcoinLedger;

// TODO: Should work with full enum `ledger::Bitcoin`

db_roundtrip_test!(
    BitcoinLedger,
    Ethereum,
    BitcoinAsset,
    Ether,
    BitcoinIdentity,
    EthereumIdentity,
    |role| {
        SwapTypes {
            alpha_ledger: LedgerKind::Bitcoin,
            beta_ledger: LedgerKind::Ethereum,
            alpha_asset: AssetKind::Bitcoin,
            beta_asset: AssetKind::Ether,
            role,
        }
    }
);

db_roundtrip_test!(
    BitcoinLedger,
    Ethereum,
    BitcoinAsset,
    Erc20,
    BitcoinIdentity,
    EthereumIdentity,
    |role| {
        SwapTypes {
            alpha_ledger: LedgerKind::Bitcoin,
            beta_ledger: LedgerKind::Ethereum,
            alpha_asset: AssetKind::Bitcoin,
            beta_asset: AssetKind::Erc20,
            role,
        }
    }
);

db_roundtrip_test!(
    Ethereum,
    BitcoinLedger,
    Ether,
    BitcoinAsset,
    EthereumIdentity,
    BitcoinIdentity,
    |role| {
        SwapTypes {
            alpha_ledger: LedgerKind::Ethereum,
            beta_ledger: LedgerKind::Bitcoin,
            alpha_asset: AssetKind::Ether,
            beta_asset: AssetKind::Bitcoin,
            role,
        }
    }
);

db_roundtrip_test!(
    Ethereum,
    BitcoinLedger,
    Erc20,
    BitcoinAsset,
    EthereumIdentity,
    BitcoinIdentity,
    |role| {
        SwapTypes {
            alpha_ledger: LedgerKind::Ethereum,
            beta_ledger: LedgerKind::Bitcoin,
            alpha_asset: AssetKind::Erc20,
            beta_asset: AssetKind::Bitcoin,
            role,
        }
    }
);
