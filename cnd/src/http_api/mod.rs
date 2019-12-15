pub mod route_factory;
pub mod routes;
#[macro_use]
pub mod impl_serialize_http;
pub mod action;
mod ethereum_network;
mod problem;
mod swap_resource;

pub use self::{
    problem::*,
    swap_resource::{SwapParameters, SwapResource, SwapStatus},
};

pub const PATH: &str = "swaps";

use crate::{
    ethereum::{self, Erc20Token},
    network::DialInformation,
    swap_protocols::{
        ledger::{self, ethereum::ChainId},
        SwapId, SwapProtocol,
    },
};
use bitcoin::util::amount::Denomination;
use libp2p::PeerId;
use libp2p_core::Multiaddr;
use serde::{
    de::{self, Error as _, MapAccess},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{
    convert::{TryFrom, TryInto},
    ops::Deref,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Http<I>(pub I);

impl<I> Deref for Http<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for Http<bitcoin::Amount> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.as_sat().to_string())
    }
}

impl<'de> Deserialize<'de> for Http<bitcoin::Amount> {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let amount = bitcoin::Amount::from_str_in(value.as_str(), Denomination::Satoshi)
            .map_err(<D as Deserializer<'de>>::Error::custom)?;

        Ok(Http(amount))
    }
}

impl Serialize for Http<bitcoin::Transaction> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.txid().to_string())
    }
}

impl Serialize for Http<crate::ethereum::Transaction> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.hash.serialize(serializer)
    }
}

impl Serialize for Http<crate::bitcoin::PublicKey> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let public_key = self.0.into_inner();
        serializer.serialize_str(&public_key.to_string())
    }
}

impl_serialize_type_with_fields!(bitcoin::OutPoint { "txid" => txid, "vout" => vout });
impl_serialize_http!(crate::ethereum::H160);
impl_serialize_http!(SwapId);

impl Serialize for Http<SwapProtocol> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.0 {
            // Currently we do not expose the hash_function protocol parameter via REST.
            SwapProtocol::Rfc003(_hash_function) => serializer.serialize_str("rfc003"),
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

impl Serialize for Http<bitcoin::Network> {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self.0 {
            bitcoin::Network::Bitcoin => "mainnet",
            bitcoin::Network::Testnet => "testnet",
            bitcoin::Network::Regtest => "regtest",
        })
    }
}

impl<'de> Deserialize<'de> for Http<bitcoin::Network> {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let network = match String::deserialize(deserializer)?.as_str() {
            "mainnet" => bitcoin::Network::Bitcoin,
            "testnet" => bitcoin::Network::Testnet,
            "regtest" => bitcoin::Network::Regtest,
            network => {
                return Err(<D as Deserializer<'de>>::Error::custom(format!(
                    "unknown network {}",
                    network
                )))
            }
        };

        Ok(Http(network))
    }
}

impl<'de> Deserialize<'de> for Http<PeerId> {
    fn deserialize<D>(deserializer: D) -> Result<Http<PeerId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let peer_id = value.parse().map_err(D::Error::custom)?;

        Ok(Http(peer_id))
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
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
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
                            address_hint = Some(map.next_value::<Multiaddr>()?)
                        }
                        _ => {
                            return Err(de::Error::unknown_field(key.as_str(), &[
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

/// An enum describing all the possible values of `alpha_ledger` and
/// `beta_ledger`.
///
/// Note: This enum makes use of serde's "try_from" and "into" feature: https://serde.rs/container-attrs.html#from
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(try_from = "HttpLedgerParams")]
#[serde(into = "HttpLedgerParams")]
pub enum HttpLedger {
    Bitcoin(ledger::Bitcoin),
    Ethereum(ledger::Ethereum),
}

/// An enum describing all the possible values of `alpha_asset` and
/// `beta_asset`.
///
/// Note: This enum makes use of serde's "try_from" and "try_into" feature: https://serde.rs/container-attrs.html#from
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(try_from = "HttpAssetParams")]
#[serde(into = "HttpAssetParams")]
pub enum HttpAsset {
    Bitcoin(bitcoin::Amount),
    Ether(ethereum::EtherQuantity),
    Erc20(ethereum::Erc20Token),
}

/// The actual enum that is used by serde to deserialize the `alpha_ledger` and
/// `beta_ledger` fields in the `SwapRequestBody`.
///
/// To achieve the format we need, we "tag" this enums variants with `name` and
/// convert all of them to lowercase. The contents of the enums are specific
/// structs that define, how we want our parameters to be deserialized.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "name")]
#[serde(rename_all = "lowercase")]
pub enum HttpLedgerParams {
    Bitcoin(BitcoinLedgerParams),
    Ethereum(EthereumLedgerParams),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BitcoinLedgerParams {
    network: Http<bitcoin::Network>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EthereumLedgerParams {
    chain_id: Option<ChainId>,
    network: Option<ethereum_network::Network>,
}

/// The actual enum that is used by serde to deserialize the `alpha_asset` and
/// `beta_asset` fields in the `SwapRequestBody`.
///
/// To achieve the format we need, we "tag" this enums variants with `name` and
/// convert all of them to lowercase. The contents of the enums are specific
/// structs that define, how we want our parameters to be deserialized.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "name")]
#[serde(rename_all = "lowercase")]
pub enum HttpAssetParams {
    Bitcoin(BitcoinAssetParams),
    Ether(EtherAssetParams),
    Erc20(Erc20AssetParams),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BitcoinAssetParams {
    quantity: Http<bitcoin::Amount>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EtherAssetParams {
    quantity: ethereum::EtherQuantity,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Erc20AssetParams {
    quantity: ethereum::Erc20Quantity,
    token_contract: ethereum::Address,
}

impl TryFrom<HttpLedgerParams> for HttpLedger {
    type Error = anyhow::Error;

    fn try_from(params: HttpLedgerParams) -> Result<Self, Self::Error> {
        Ok(match params {
            HttpLedgerParams::Bitcoin(params) => HttpLedger::Bitcoin(params.into()),
            HttpLedgerParams::Ethereum(params) => HttpLedger::Ethereum(params.try_into()?),
        })
    }
}

impl From<HttpLedger> for HttpLedgerParams {
    fn from(ledger: HttpLedger) -> Self {
        match ledger {
            HttpLedger::Bitcoin(ledger) => HttpLedgerParams::Bitcoin(ledger.into()),
            HttpLedger::Ethereum(ledger) => HttpLedgerParams::Ethereum(ledger.into()),
        }
    }
}

impl From<BitcoinLedgerParams> for ledger::Bitcoin {
    fn from(params: BitcoinLedgerParams) -> Self {
        Self {
            network: *params.network,
        }
    }
}

impl From<ledger::Bitcoin> for BitcoinLedgerParams {
    fn from(bitcoin: ledger::Bitcoin) -> Self {
        Self {
            network: Http(bitcoin.network),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("The Ethereum ledger requires either a network or a chain-id parameter.")]
pub struct InvalidEthereumLedgerParams;

impl TryFrom<EthereumLedgerParams> for ledger::Ethereum {
    type Error = InvalidEthereumLedgerParams;

    fn try_from(params: EthereumLedgerParams) -> Result<Self, Self::Error> {
        let chain_id = match params {
            EthereumLedgerParams {
                network: Some(network),
                ..
            } => network.into(),
            EthereumLedgerParams {
                chain_id: Some(chain_id),
                ..
            } => chain_id,
            EthereumLedgerParams {
                network: None,
                chain_id: None,
            } => return Err(InvalidEthereumLedgerParams),
        };

        Ok(Self { chain_id })
    }
}

impl From<ledger::Ethereum> for EthereumLedgerParams {
    fn from(ethereum: ledger::Ethereum) -> Self {
        let chain_id = ethereum.chain_id;

        Self {
            network: chain_id.try_into().ok(),
            chain_id: Some(chain_id),
        }
    }
}

impl From<HttpAssetParams> for HttpAsset {
    fn from(params: HttpAssetParams) -> Self {
        match params {
            HttpAssetParams::Bitcoin(params) => HttpAsset::Bitcoin(params.into()),
            HttpAssetParams::Ether(params) => HttpAsset::Ether(params.into()),
            HttpAssetParams::Erc20(params) => HttpAsset::Erc20(params.into()),
        }
    }
}

impl From<HttpAsset> for HttpAssetParams {
    fn from(asset: HttpAsset) -> Self {
        match asset {
            HttpAsset::Bitcoin(asset) => HttpAssetParams::Bitcoin(asset.into()),
            HttpAsset::Ether(asset) => HttpAssetParams::Ether(asset.into()),
            HttpAsset::Erc20(asset) => HttpAssetParams::Erc20(asset.into()),
        }
    }
}

impl From<BitcoinAssetParams> for bitcoin::Amount {
    fn from(params: BitcoinAssetParams) -> Self {
        *params.quantity
    }
}

impl From<bitcoin::Amount> for BitcoinAssetParams {
    fn from(bitcoin: bitcoin::Amount) -> Self {
        Self {
            quantity: Http(bitcoin),
        }
    }
}

impl From<EtherAssetParams> for ethereum::EtherQuantity {
    fn from(params: EtherAssetParams) -> Self {
        params.quantity
    }
}

impl From<ethereum::EtherQuantity> for EtherAssetParams {
    fn from(ether: ethereum::EtherQuantity) -> Self {
        Self { quantity: ether }
    }
}

impl From<Erc20AssetParams> for ethereum::Erc20Token {
    fn from(params: Erc20AssetParams) -> Self {
        Self {
            token_contract: params.token_contract,
            quantity: params.quantity,
        }
    }
}

impl From<ethereum::Erc20Token> for Erc20AssetParams {
    fn from(erc20: Erc20Token) -> Self {
        Self {
            quantity: erc20.quantity,
            token_contract: erc20.token_contract,
        }
    }
}

impl From<ledger::Bitcoin> for HttpLedger {
    fn from(bitcoin: ledger::Bitcoin) -> Self {
        HttpLedger::Bitcoin(bitcoin)
    }
}

impl From<ledger::Ethereum> for HttpLedger {
    fn from(ethereum: ledger::Ethereum) -> Self {
        HttpLedger::Ethereum(ethereum)
    }
}

impl From<bitcoin::Amount> for HttpAsset {
    fn from(bitcoin: bitcoin::Amount) -> Self {
        HttpAsset::Bitcoin(bitcoin)
    }
}

impl From<ethereum::EtherQuantity> for HttpAsset {
    fn from(ether: ethereum::EtherQuantity) -> Self {
        HttpAsset::Ether(ether)
    }
}

impl From<ethereum::Erc20Token> for HttpAsset {
    fn from(erc20: ethereum::Erc20Token) -> Self {
        HttpAsset::Erc20(erc20)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ethereum::{Erc20Quantity, Erc20Token, EtherQuantity, H160, H256, U256},
        http_api::{Http, HttpAsset, HttpLedger},
        swap_protocols::{
            ledger::{ethereum, Bitcoin, Ethereum},
            HashFunction, SwapId, SwapProtocol,
        },
    };
    use bitcoin::{
        hashes::{hex::FromHex, sha256d},
        OutPoint, Script, TxIn,
    };
    use libp2p::PeerId;
    use std::str::FromStr;

    #[test]
    fn http_asset_serializes_correctly_to_json() {
        let bitcoin = HttpAsset::from(bitcoin::Amount::from_btc(1.0).unwrap());
        let ether = HttpAsset::from(EtherQuantity::from_eth(1.0));
        let pay = HttpAsset::from(Erc20Token::new(
            "B97048628DB6B661D4C2aA833e95Dbe1A905B280".parse().unwrap(),
            Erc20Quantity(U256::from(100_000_000_000u64)),
        ));

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
    fn bitcoin_http_ledger_regtest_serializes_correctly_to_json() {
        let input = &[
            HttpLedger::from(Bitcoin::new(bitcoin::Network::Bitcoin)),
            HttpLedger::from(Bitcoin::new(bitcoin::Network::Testnet)),
            HttpLedger::from(Bitcoin::new(bitcoin::Network::Regtest)),
        ];

        let expected = &[
            r#"{"name":"bitcoin","network":"mainnet"}"#,
            r#"{"name":"bitcoin","network":"testnet"}"#,
            r#"{"name":"bitcoin","network":"regtest"}"#,
        ];

        let actual = input
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<String>, serde_json::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn ethereum_http_ledger_regtest_serializes_correctly_to_json() {
        let input = &[
            HttpLedger::from(Ethereum::new(ethereum::ChainId::new(1))),
            HttpLedger::from(Ethereum::new(ethereum::ChainId::new(3))),
            HttpLedger::from(Ethereum::new(ethereum::ChainId::new(17))),
        ];

        let expected = &[
            r#"{"name":"ethereum","chain_id":1,"network":"mainnet"}"#,
            r#"{"name":"ethereum","chain_id":3,"network":"ropsten"}"#,
            r#"{"name":"ethereum","chain_id":17,"network":"regtest"}"#,
        ];

        let actual = input
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<String>, serde_json::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn http_transaction_serializes_correctly_to_json() {
        let bitcoin_tx = bitcoin::Transaction {
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
        let ethereum_tx = crate::ethereum::Transaction {
            hash: H256::repeat_byte(1),
            ..crate::ethereum::Transaction::default()
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
        let bitcoin_identity = crate::bitcoin::PublicKey::new(
            "02ef606e64a51b07373f81e042887e8e9c3806f0ff3fe3711df18beba8b82d82e6"
                .parse()
                .unwrap(),
        );

        let ethereum_identity = H160::repeat_byte(7);

        let bitcoin_identity = Http(bitcoin_identity);
        let ethereum_identity = Http(ethereum_identity);

        let bitcoin_identity_serialized = serde_json::to_string(&bitcoin_identity).unwrap();
        let ethereum_identity_serialized = serde_json::to_string(&ethereum_identity).unwrap();

        assert_eq!(
            &bitcoin_identity_serialized,
            r#""02ef606e64a51b07373f81e042887e8e9c3806f0ff3fe3711df18beba8b82d82e6""#
        );
        assert_eq!(
            &ethereum_identity_serialized,
            r#""0x0707070707070707070707070707070707070707""#
        );
    }

    #[test]
    fn http_htlc_location_serializes_correctly_to_json() {
        let bitcoin_htlc_location = OutPoint {
            txid: sha256d::Hash::from_hex(
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
