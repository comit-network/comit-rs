pub mod rfc003;
pub mod route_factory;

#[macro_use]
pub mod ledger;

#[macro_use]
pub mod asset;

pub const PATH: &str = "swaps";

mod ledger_impls {
    use super::ledger::{Error, FromHttpLedger, HttpLedger, ToHttpLedger};
    use swap_protocols::ledger::{Bitcoin, Ethereum};

    impl_http_ledger!(Bitcoin { network });
    impl_http_ledger!(Ethereum);

}

mod asset_impls {
    use super::asset::{Error, FromHttpAsset, HttpAsset, ToHttpAsset};
    use bitcoin_support::BitcoinQuantity;
    use ethereum_support::{Erc20Quantity, EtherQuantity};

    impl_http_quantity_asset!(BitcoinQuantity, Bitcoin);
    impl_http_quantity_asset!(EtherQuantity, Ether);

    impl FromHttpAsset for Erc20Quantity {
        fn from_http_asset(_asset: HttpAsset) -> Result<Self, Error> {
            unimplemented!(
                "allow to specify only the asset name here, read the rest from the config"
            )
        }
    }

    impl ToHttpAsset for Erc20Quantity {
        fn to_http_asset(&self) -> Result<HttpAsset, Error> {
            unimplemented!(
                "allow to specify only the asset name here, read the rest from the config"
            )
        }
    }
}
