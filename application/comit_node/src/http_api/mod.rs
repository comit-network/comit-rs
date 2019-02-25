pub mod rfc003;
pub mod route_factory;

#[macro_use]
pub mod ledger;
#[macro_use]
pub mod asset;
#[macro_use]
pub mod impl_serialize_http;

mod problem;

pub use self::problem::*;

pub const PATH: &str = "swaps";

use crate::{
    connection_pool::ConnectionPool,
    http_api::{
        asset::{FromHttpAsset, HttpAsset},
        ledger::{FromHttpLedger, HttpLedger},
    },
    swap_protocols::ledger::{Bitcoin, Ethereum},
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Token, EtherQuantity};
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::{net::SocketAddr, sync::Arc};
use warp::{self, Rejection, Reply};

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

#[derive(Debug)]
pub struct Http<I>(pub I);

impl_serialize_http!(Bitcoin { "network" => network });
impl_from_http_ledger!(Bitcoin { network });
impl_serialize_http!(BitcoinQuantity := "Bitcoin" { "quantity" });
impl_from_http_quantity_asset!(BitcoinQuantity, Bitcoin);

impl Serialize for Http<bitcoin_support::Transaction> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self.0.txid()))
    }
}

impl Serialize for Http<bitcoin_support::PubkeyHash> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl_serialize_http!(Ethereum { "network" => network });
impl_from_http_ledger!(Ethereum { network });
impl_serialize_http!(EtherQuantity := "Ether" { "quantity" });
impl_serialize_http!(Erc20Token := "ERC20" { "quantity" => quantity, "token_contract" => token_contract });
impl_from_http_quantity_asset!(EtherQuantity, Ether);

impl FromHttpAsset for Erc20Token {
    fn from_http_asset(mut asset: HttpAsset) -> Result<Self, asset::Error> {
        asset.is_asset("ERC20")?;

        Ok(Erc20Token::new(
            asset.parameter("token_contract")?,
            asset.parameter("quantity")?,
        ))
    }
}

impl Serialize for Http<ethereum_support::Transaction> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.hash.serialize(serializer)
    }
}

impl Serialize for Http<ethereum_support::H160> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http_api::Http,
        swap_protocols::ledger::{Bitcoin, Ethereum},
    };
    use bitcoin_support::{self, BitcoinQuantity, OutPoint, PubkeyHash, Script, TxIn};
    use ethereum_support::{
        self, Address, Bytes, Erc20Quantity, Erc20Token, EtherQuantity, H160, H256, U256,
    };

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

    #[test]
    fn http_transaction_serializes_correctly_to_json() {
        let bitcoin_tx = bitcoin_support::Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::null(),
                script_sig: Script::new(),
                sequence: 0,
                witness: vec![],
            }],
            output: vec![],
        };
        let ethereum_tx = ethereum_support::Transaction {
            hash: H256::from(348924802),
            nonce: U256::from(0),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: H160::from(0),
            to: None,
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![]),
        };

        let bitcoin_tx = Http(bitcoin_tx);
        let ethereum_tx = Http(ethereum_tx);

        let bitcoin_tx_serialized = serde_json::to_string(&bitcoin_tx).unwrap();
        let ethereum_tx_serialized = serde_json::to_string(&ethereum_tx).unwrap();

        assert_eq!(
            &bitcoin_tx_serialized,
            r#""e6634b155d7d472f60629d168f612781efa9f48e256c5aa3f9ddd2fa181fdedf""#
        );
        assert_eq!(
            &ethereum_tx_serialized,
            r#""0x0000000000000000000000000000000000000000000000000000000014cc2b82""#
        );
    }

    #[test]
    fn http_identity_serializes_correctly_to_json() {
        let bitcoin_identity: Vec<u8> =
            hex::decode("c021f17be99c6adfbcba5d38ee0d292c0399d2f5").unwrap();
        let bitcoin_identity = PubkeyHash::from(&bitcoin_identity[..]);
        let ethereum_identity = H160::from(7);

        let bitcoin_identity = Http(bitcoin_identity);
        let ethereum_identity = Http(ethereum_identity);

        let bitcoin_identity_serialized = serde_json::to_string(&bitcoin_identity).unwrap();
        let ethereum_identity_serialized = serde_json::to_string(&ethereum_identity).unwrap();

        assert_eq!(
            &bitcoin_identity_serialized,
            r#""c021f17be99c6adfbcba5d38ee0d292c0399d2f5""#
        );
        assert_eq!(
            &ethereum_identity_serialized,
            r#""0x0000000000000000000000000000000000000007""#
        );
    }

}
