use crate::{
    db::{load_swaps::LoadAcceptedSwap, Location, SaveMessage, Sqlite},
    quickcheck::Quickcheck,
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{Accept, Request},
        SwapId,
    },
};
use bitcoin::Amount as BitcoinAmount;
use ethereum_support::{Erc20Token, EtherQuantity};

macro_rules! db_roundtrip_test {
    ($alpha_ledger:ident, $beta_ledger:ident, $alpha_asset:ident, $beta_asset:ident) => {
        paste::item! {
            #[test]
            #[allow(non_snake_case)]
            fn [<roundtrip_test_ $alpha_ledger _ $beta_ledger _ $alpha_asset _ $beta_asset>]() {
                fn prop(swap_id: Quickcheck<SwapId>,
                    request: Quickcheck<Request<$alpha_ledger, $beta_ledger, $alpha_asset, $beta_asset>>,
                    accept: Quickcheck<Accept<$alpha_ledger, $beta_ledger>>) -> anyhow::Result<bool> {
                let db_path = tempfile::NamedTempFile::new()?.into_temp_path();
                let db = Sqlite::new(Location::OnDisk(&db_path))?;

                let saved_request = Request {
                    swap_id: *swap_id,
                    ..*request
                };
                let saved_accept = Accept {
                    swap_id: *swap_id,
                    ..*accept
                };

                db.save_message(saved_request.clone())?;
                db.save_message(saved_accept.clone())?;

                let (loaded_request, loaded_accept) = db.load_accepted_swap(*swap_id)?;

                Ok(saved_request == loaded_request && saved_accept == loaded_accept)
            }

            quickcheck::quickcheck(prop as fn(Quickcheck<SwapId>, Quickcheck<Request<$alpha_ledger, $beta_ledger, $alpha_asset, $beta_asset>>, Quickcheck<Accept<$alpha_ledger, $beta_ledger>>) -> anyhow::Result<bool>);
            }
        }
    };
}

db_roundtrip_test!(Bitcoin, Ethereum, BitcoinAmount, EtherQuantity);
db_roundtrip_test!(Ethereum, Bitcoin, EtherQuantity, BitcoinAmount);
db_roundtrip_test!(Bitcoin, Ethereum, BitcoinAmount, Erc20Token);
db_roundtrip_test!(Ethereum, Bitcoin, Erc20Token, BitcoinAmount);
