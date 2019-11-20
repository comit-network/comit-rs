use crate::{
    db::{
        load_swaps::LoadAcceptedSwap,
        swap_types::{DetermineTypes, SwapTypes},
        AssetKind, LedgerKind, Retrieve, Save, SaveMessage, Sqlite, Swap,
    },
    quickcheck::Quickcheck,
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{Accept, Request},
    },
};
use bitcoin::Amount as BitcoinAmount;
use ethereum_support::{Erc20Token, EtherQuantity};

macro_rules! db_roundtrip_test {
    ($alpha_ledger:ident, $beta_ledger:ident, $alpha_asset:ident, $beta_asset:ident, $expected_swap_types_fn:expr) => {
        paste::item! {
            #[test]
            #[allow(non_snake_case, clippy::redundant_closure_call)]
            fn [<roundtrip_test_ $alpha_ledger _ $beta_ledger _ $alpha_asset _ $beta_asset>]() {
                fn prop(swap: Quickcheck<Swap>,
                        request: Quickcheck<Request<$alpha_ledger, $beta_ledger, $alpha_asset, $beta_asset>>,
                        accept: Quickcheck<Accept<$alpha_ledger, $beta_ledger>>,
                ) -> anyhow::Result<bool> {

                    // unpack the swap from the generic newtype
                    let Swap { swap_id, role, counterparty } = swap.0;

                    // construct the expected swap types from the function we get passed in order to enrich it with the role
                    let expected_swap_types = ($expected_swap_types_fn)(role);

                    let db_path = tempfile::Builder::new()
                        .prefix(&swap_id.to_string())
                        .suffix(".sqlite")
                        .tempfile()?
                        .into_temp_path();

                    let db = Sqlite::new(&db_path)?;

                    let saved_swap = Swap {
                        swap_id,
                        role,
                        counterparty
                    };
                    let saved_request = Request {
                        swap_id,
                        ..*request
                    };
                    let saved_accept = Accept {
                        swap_id,
                        ..*accept
                    };

                    db.save(saved_swap.clone())?;
                    db.save_message(saved_request.clone())?;
                    db.save_message(saved_accept.clone())?;

                    let loaded_swap = Retrieve::get(&db, &swap_id)?;
                    let (loaded_request, loaded_accept) = db.load_accepted_swap(swap_id)?;
                    let loaded_swap_types = db.determine_types(&swap_id)?;

                    Ok(
                        saved_request == loaded_request &&
                        saved_accept == loaded_accept &&
                        loaded_swap == saved_swap &&
                        loaded_swap_types == expected_swap_types
                    )
                }

                quickcheck::quickcheck(prop as fn(
                    Quickcheck<Swap>,
                    Quickcheck<Request<$alpha_ledger, $beta_ledger, $alpha_asset, $beta_asset>>,
                    Quickcheck<Accept<$alpha_ledger, $beta_ledger>>,
                ) -> anyhow::Result<bool>);
            }
        }
    };
}

db_roundtrip_test!(Bitcoin, Ethereum, BitcoinAmount, EtherQuantity, |role| {
    SwapTypes {
        alpha_ledger: LedgerKind::Bitcoin,
        beta_ledger: LedgerKind::Ethereum,
        alpha_asset: AssetKind::Bitcoin,
        beta_asset: AssetKind::Ether,
        role,
    }
});
db_roundtrip_test!(Ethereum, Bitcoin, EtherQuantity, BitcoinAmount, |role| {
    SwapTypes {
        alpha_ledger: LedgerKind::Ethereum,
        beta_ledger: LedgerKind::Bitcoin,
        alpha_asset: AssetKind::Ether,
        beta_asset: AssetKind::Bitcoin,
        role,
    }
});
db_roundtrip_test!(Bitcoin, Ethereum, BitcoinAmount, Erc20Token, |role| {
    SwapTypes {
        alpha_ledger: LedgerKind::Bitcoin,
        beta_ledger: LedgerKind::Ethereum,
        alpha_asset: AssetKind::Bitcoin,
        beta_asset: AssetKind::Erc20,
        role,
    }
});
db_roundtrip_test!(Ethereum, Bitcoin, Erc20Token, BitcoinAmount, |role| {
    SwapTypes {
        alpha_ledger: LedgerKind::Ethereum,
        beta_ledger: LedgerKind::Bitcoin,
        alpha_asset: AssetKind::Erc20,
        beta_asset: AssetKind::Bitcoin,
        role,
    }
});
