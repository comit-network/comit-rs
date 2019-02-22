pub mod rfc003;
pub mod route_factory;

#[macro_use]
pub mod ledger;
#[macro_use]
pub mod asset;

mod problem;

pub use self::problem::*;

pub const PATH: &str = "swaps";

use crate::connection_pool::ConnectionPool;
use std::{net::SocketAddr, sync::Arc};
use warp::{self, Rejection, Reply};

mod ledger_impls {
    use super::ledger::{Error, FromHttpLedger, HttpLedger};
    use crate::swap_protocols::ledger::{Bitcoin, Ethereum};

    impl_from_http_ledger!(Bitcoin { network });
    impl_from_http_ledger!(Ethereum { network });

}

mod asset_impls {
    use super::asset::{Error, FromHttpAsset, HttpAsset};
    use bitcoin_support::BitcoinQuantity;
    use ethereum_support::{Erc20Token, EtherQuantity};

    impl_from_http_quantity_asset!(BitcoinQuantity, Bitcoin);
    impl_from_http_quantity_asset!(EtherQuantity, Ether);

    impl FromHttpAsset for Erc20Token {
        fn from_http_asset(mut asset: HttpAsset) -> Result<Self, Error> {
            asset.is_asset("ERC20")?;

            Ok(Erc20Token::new(
                asset.parameter("token_contract")?,
                asset.parameter("quantity")?,
            ))
        }
    }
}

#[derive(Debug, Serialize)]
struct GetPeers {
    pub peers: Vec<SocketAddr>,
}

pub fn peers(connection_pool: Arc<ConnectionPool>) -> Result<impl Reply, Rejection> {
    let response = GetPeers {
        peers: connection_pool.connected_addrs(),
    };

    Ok(warp::reply::json(&response))
}

#[cfg(test)]
mod tests {

    use crate::{
        http_api::rfc003::handlers::Http,
        swap_protocols::ledger::{Bitcoin, Ethereum},
    };
    use bitcoin_support::{self, BitcoinQuantity};
    use ethereum_support::{self, Address, Erc20Quantity, Erc20Token, EtherQuantity, U256};

    #[test]
    fn http_asset_serializes_correctly_to_json() {
        let bitcoin = BitcoinQuantity::from_bitcoin(1.0);
        let ether = EtherQuantity::from_eth(1.0);
        let pay = Erc20Token::new(
            Address::from("0xB97048628DB6B661D4C2aA833e95Dbe1A905B280"),
            Erc20Quantity(U256::from(100_000_000_000u64)),
        );

        let bitcoin = Http(bitcoin);
        let ether = Http(ether);
        let pay = Http(pay);

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
        let bitcoin = Bitcoin::new(bitcoin_support::Network::Regtest);
        let ethereum = Ethereum::new(ethereum_support::Network::Regtest);

        let bitcoin = Http(bitcoin);
        let ethereum = Http(ethereum);

        let bitcoin_serialized = serde_json::to_string(&bitcoin).unwrap();
        let ethereum_serialized = serde_json::to_string(&ethereum).unwrap();

        assert_eq!(
            &bitcoin_serialized,
            r#"{"name":"Bitcoin","network":"regtest"}"#
        );
        assert_eq!(
            &ethereum_serialized,
            r#"{"name":"Ethereum","network":"regtest"}"#
        );
    }

}
