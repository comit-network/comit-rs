#[macro_use]
pub mod with_swap_types;
pub mod route_factory;
pub mod routes;
pub mod serde_peer_id;
#[macro_use]
pub mod ledger;
#[macro_use]
pub mod asset;
#[macro_use]
pub mod impl_serialize_http;
pub mod action;
mod problem;
mod swap_resource;

pub use self::{
    problem::*,
    swap_resource::{SwapParameters, SwapResource, SwapStatus},
};

pub const PATH: &str = "swaps";

use crate::{
    http_api::{
        asset::{FromHttpAsset, HttpAsset},
        ledger::{FromHttpLedger, HttpLedger},
    },
    network::DialInformation,
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        SwapId, SwapProtocol,
    },
};
use bitcoin_support::{amount::Denomination, Amount as BitcoinAmount};
use ethereum_support::{Erc20Token, EtherQuantity};
use libp2p::PeerId;
use serde::{
    de::{self, MapAccess},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};

#[derive(Debug)]
pub struct Http<I>(pub I);

impl_serialize_type_name_with_fields!(Bitcoin { "network" => network });
impl_from_http_ledger!(Bitcoin { network });

impl FromHttpAsset for BitcoinAmount {
    fn from_http_asset(mut asset: HttpAsset) -> Result<Self, asset::Error> {
        let name = String::from("bitcoin");
        asset.is_asset(name.as_ref())?;

        let quantity = asset.parameter::<String>("quantity")?;

        BitcoinAmount::from_str_in(quantity.as_str(), Denomination::Satoshi)
            .map_err(|_| asset::Error::Parsing)
    }
}

impl Serialize for Http<BitcoinAmount> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("", 2)?;
        state.serialize_field("name", "bitcoin")?;
        state.serialize_field("quantity", &self.0.as_sat().to_string())?;
        state.end()
    }
}

impl Serialize for Http<bitcoin_support::Transaction> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self.0.txid()))
    }
}

impl_serialize_type_name_with_fields!(Ethereum { "network" => network });
impl_from_http_ledger!(Ethereum { network });
impl_serialize_type_name_with_fields!(EtherQuantity := "ether" { "quantity" });
impl_serialize_type_name_with_fields!(Erc20Token := "erc20" { "quantity" => quantity, "token_contract" => token_contract });
impl_from_http_quantity_asset!(EtherQuantity, Ether);

impl FromHttpAsset for Erc20Token {
    fn from_http_asset(mut asset: HttpAsset) -> Result<Self, asset::Error> {
        asset.is_asset("erc20")?;

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

impl_serialize_http!(bitcoin_support::PubkeyHash);
impl_serialize_type_with_fields!(bitcoin_support::OutPoint { "txid" => txid, "vout" => vout });
impl_serialize_http!(ethereum_support::H160);
impl_serialize_http!(SwapId);

impl Serialize for Http<SwapProtocol> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.0 {
            // Currently we do not expose the hash_function protocol parameter via REST.
            SwapProtocol::Rfc003(_hash_function) => serializer.serialize_str("rfc003"),
            SwapProtocol::Unknown(name) => serializer.serialize_str(name.as_str()),
        }
    }
}

impl Serialize for Http<PeerId> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_base58()[..])
    }
}

impl<'de> Deserialize<'de> for Http<PeerId> {
    fn deserialize<D>(deserializer: D) -> Result<Http<PeerId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Http<PeerId>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a peer id")
            }

            fn visit_str<E>(self, value: &str) -> Result<Http<PeerId>, E>
            where
                E: de::Error,
            {
                let peer_id = value.parse().map_err(E::custom)?;
                Ok(Http(peer_id))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl<'de> Deserialize<'de> for DialInformation {
    fn deserialize<D>(deserializer: D) -> Result<DialInformation, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = DialInformation;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a peer id or a dial information struct")
            }

            fn visit_str<E>(self, value: &str) -> Result<DialInformation, E>
            where
                E: de::Error,
            {
                let peer_id = value.parse().map_err(E::custom)?;
                Ok(DialInformation {
                    peer_id,
                    address_hint: None,
                })
            }

            fn visit_map<M>(self, mut map: M) -> Result<DialInformation, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut peer_id = None;
                let mut address_hint = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        "peer_id" => {
                            if peer_id.is_some() {
                                return Err(de::Error::duplicate_field("peer_id"));
                            }
                            peer_id = Some(map.next_value::<Http<PeerId>>()?)
                        }
                        "address_hint" => {
                            if address_hint.is_some() {
                                return Err(de::Error::duplicate_field("address_hint"));
                            }
                            address_hint = Some(map.next_value()?)
                        }
                        _ => {
                            return Err(de::Error::unknown_field(key, &[
                                "peer_id",
                                "address_hint",
                            ]));
                        }
                    }
                }
                let peer_id = peer_id.ok_or_else(|| de::Error::missing_field("peer_id"))?;
                Ok(DialInformation {
                    peer_id: peer_id.0,
                    address_hint,
                })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        http_api::Http,
        swap_protocols::{
            ledger::{Bitcoin, Ethereum},
            HashFunction, SwapId, SwapProtocol,
        },
    };
    use bitcoin_support::{self, FromHex, OutPoint, PubkeyHash, Script, Sha256dHash, TxIn};
    use ethereum_support::{self, Erc20Quantity, Erc20Token, EtherQuantity, H160, H256, U256};
    use libp2p::PeerId;
    use std::{convert::TryFrom, str::FromStr};

    #[test]
    fn http_asset_serializes_correctly_to_json() {
        let bitcoin = BitcoinAmount::from_btc(1.0).unwrap();
        let ether = EtherQuantity::from_eth(1.0);
        let pay = Erc20Token::new(
            "B97048628DB6B661D4C2aA833e95Dbe1A905B280".parse().unwrap(),
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
            r#"{"name":"bitcoin","quantity":"100000000"}"#
        );
        assert_eq!(
            &ether_serialized,
            r#"{"name":"ether","quantity":"1000000000000000000"}"#
        );
        assert_eq!(&pay_serialized, r#"{"name":"erc20","quantity":"100000000000","token_contract":"0xb97048628db6b661d4c2aa833e95dbe1a905b280"}"#);
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
            r#"{"name":"bitcoin","network":"regtest"}"#
        );
        assert_eq!(
            &ethereum_serialized,
            r#"{"name":"ethereum","network":"regtest"}"#
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
            hash: H256::repeat_byte(1),
            ..ethereum_support::Transaction::default()
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
            r#""0x0101010101010101010101010101010101010101010101010101010101010101""#
        );
    }

    #[test]
    fn http_identity_serializes_correctly_to_json() {
        let bitcoin_identity: Vec<u8> =
            hex::decode("c021f17be99c6adfbcba5d38ee0d292c0399d2f5").unwrap();
        let bitcoin_identity = PubkeyHash::try_from(&bitcoin_identity[..]).unwrap();
        let ethereum_identity = H160::repeat_byte(7);

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
            r#""0x0707070707070707070707070707070707070707""#
        );
    }

    #[test]
    fn http_htlc_location_serializes_correctly_to_json() {
        let bitcoin_htlc_location = OutPoint {
            txid: Sha256dHash::from_hex(
                "ad067ee417ee5518122374307d1fa494c67e30c75d38c7061d944b59e56fe024",
            )
            .unwrap(),
            vout: 1u32,
        };
        // Ethereum HtlcLocation matches Ethereum Identity, so it is already being
        // tested elsewhere.

        let bitcoin_htlc_location = Http(bitcoin_htlc_location);

        let bitcoin_htlc_location_serialized =
            serde_json::to_string(&bitcoin_htlc_location).unwrap();

        assert_eq!(
            &bitcoin_htlc_location_serialized,
            r#"{"txid":"ad067ee417ee5518122374307d1fa494c67e30c75d38c7061d944b59e56fe024","vout":1}"#
        );
    }

    #[test]
    fn http_swap_protocol_serializes_correctly_to_json() {
        let protocol = SwapProtocol::Rfc003(HashFunction::Sha256);
        let protocol = Http(protocol);
        let serialized = serde_json::to_string(&protocol).unwrap();
        assert_eq!(serialized, r#""rfc003""#);
    }

    #[test]
    fn http_swap_id_serializes_correctly_to_json() {
        let swap_id = SwapId::from_str("ad2652ca-ecf2-4cc6-b35c-b4351ac28a34").unwrap();
        let swap_id = Http(swap_id);

        let swap_id_serialized = serde_json::to_string(&swap_id).unwrap();
        assert_eq!(
            swap_id_serialized,
            r#""ad2652ca-ecf2-4cc6-b35c-b4351ac28a34""#
        )
    }

    #[test]
    fn http_peer_id_serializes_correctly_to_json() {
        let peer_id = PeerId::from_str("QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY").unwrap();
        let peer_id = Http(peer_id);

        let serialized = serde_json::to_string(&peer_id).unwrap();
        assert_eq!(
            serialized,
            r#""QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY""#
        );
    }
}
