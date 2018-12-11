pub mod rfc003;
pub mod route_factory;

#[macro_use]
pub mod ledger;

#[macro_use]
pub mod asset;

#[macro_use]
pub mod lock_duration;

mod problem;

pub use self::problem::*;

pub const PATH: &str = "swaps";

mod ledger_impls {
    use super::ledger::{Error, FromHttpLedger, HttpLedger, ToHttpLedger};
    use crate::swap_protocols::ledger::{Bitcoin, Ethereum};

    impl_http_ledger!(Bitcoin { network });
    impl_http_ledger!(Ethereum);

}

mod asset_impls {
    use super::asset::{Error, FromHttpAsset, HttpAsset, ToHttpAsset};
    use bitcoin_support::BitcoinQuantity;
    use ethereum_support::{
        web3::types::U256, Erc20Quantity, EtherQuantity, FromDecimalStr, ToBigDecimal,
    };

    impl_http_quantity_asset!(BitcoinQuantity, Bitcoin);
    impl_http_quantity_asset!(EtherQuantity, Ether);

    impl FromHttpAsset for Erc20Quantity {
        fn from_http_asset(mut asset: HttpAsset) -> Result<Self, Error> {
            asset.is_asset("ERC20")?;

            let amount: String = asset.parameter("quantity")?;

            Ok(Erc20Quantity::new(
                asset.parameter("token_contract")?,
                U256::from_decimal_str(&amount).map_err(|_| Error::Parsing)?,
            ))
        }
    }

    impl ToHttpAsset for Erc20Quantity {
        fn to_http_asset(&self) -> Result<HttpAsset, Error> {
            Ok(HttpAsset::with_asset("ERC20")
                .with_parameter("quantity", format!("{}", self.quantity().to_bigdec(0)))?
                .with_parameter("token_contract", self.token_contract())?)
        }
    }
}

mod lock_duration_impls {
    use super::lock_duration::{Error, HttpLockDuration, ToHttpLockDuration};
    use crate::swap_protocols::rfc003::ethereum::Seconds;
    use bitcoin_support::Blocks;

    impl_to_http_lock_duration!(Blocks);
    impl_to_http_lock_duration!(Seconds);
}

#[cfg(test)]
mod tests {

    use crate::{
        http_api::{asset::ToHttpAsset, ledger::ToHttpLedger},
        swap_protocols::ledger::{Bitcoin, Ethereum},
    };
    use bitcoin_support::{BitcoinQuantity, Network};
    use ethereum_support::{Address, Erc20Quantity, EtherQuantity, U256};

    #[test]
    fn http_asset_serializes_correctly_to_json() {
        let bitcoin = BitcoinQuantity::from_bitcoin(1.0);
        let ether = EtherQuantity::from_eth(1.0);
        let pay = Erc20Quantity::new(
            Address::from("0xB97048628DB6B661D4C2aA833e95Dbe1A905B280"),
            U256::from(100_000_000_000u64),
        );

        let bitcoin = bitcoin.to_http_asset().unwrap();
        let ether = ether.to_http_asset().unwrap();
        let pay = pay.to_http_asset().unwrap();

        let bitcoin_serialized = serde_json::to_string(&bitcoin).unwrap();
        let ether_serialized = serde_json::to_string(&ether).unwrap();
        let pay_serialized = serde_json::to_string(&pay).unwrap();

        assert_eq!(
            &bitcoin_serialized,
            r#"{"name":"Bitcoin","quantity":"100000000"}"#
        );
        assert_eq!(
            &ether_serialized,
            r#"{"name":"Ether","quantity":"1000000000000000000"}"#
        );
        assert_eq!(&pay_serialized, r#"{"name":"ERC20","quantity":"100000000000","token_contract":"0xb97048628db6b661d4c2aa833e95dbe1a905b280"}"#);
    }

    #[test]
    fn http_ledger_serializes_correctly_to_json() {
        let bitcoin = Bitcoin {
            network: Network::Regtest,
        };
        let ethereum = Ethereum {};

        let bitcoin = bitcoin.to_http_ledger().unwrap();
        let ethereum = ethereum.to_http_ledger().unwrap();

        let bitcoin_serialized = serde_json::to_string(&bitcoin).unwrap();
        let ethereum_serialized = serde_json::to_string(&ethereum).unwrap();

        assert_eq!(
            &bitcoin_serialized,
            r#"{"name":"Bitcoin","network":"regtest"}"#
        );
        assert_eq!(&ethereum_serialized, r#"{"name":"Ethereum"}"#);
    }

}
