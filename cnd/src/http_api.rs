pub mod halight;
pub mod halight_herc20;
pub mod hbit;
pub mod hbit_herc20;
pub mod herc20;
pub mod herc20_halight;
pub mod herc20_hbit;
pub mod route_factory;
pub mod routes;
#[macro_use]
pub mod impl_serialize_http;
pub mod action;
mod problem;
mod protocol;
mod swap_resource;

pub use self::{
    problem::*,
    protocol::{AliceSwap, BobSwap},
    swap_resource::{OnFail, SwapParameters, SwapResource, SwapStatus},
};
use crate::swap_protocols::actions::lnd::Chain;

pub const PATH: &str = "swaps";

use crate::{
    asset,
    ethereum::ChainId,
    htlc_location, identity,
    swap_protocols::{ledger, rfc003::SwapId, SwapProtocol},
    transaction, Role,
};
use libp2p::{Multiaddr, PeerId};
use serde::{
    de::Error as _, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer,
};
use std::{
    convert::{TryFrom, TryInto},
    ops::Deref,
    str::FromStr,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Swap<A, B> {
    pub role: Role,
    pub alpha: A,
    pub beta: B,
    /* pub status: SwapStatus (if you want to have this, you need to save ledger state to the
     * database) */
}

#[derive(Clone, Debug, PartialEq)]
pub struct Http<I>(pub I);

impl<I> From<I> for Http<I> {
    fn from(inner: I) -> Self {
        Http(inner)
    }
}

impl<I> Deref for Http<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for Http<Role> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Http<Role> {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let role = String::deserialize(deserializer)?;
        let role =
            Role::from_str(role.as_str()).map_err(<D as Deserializer<'de>>::Error::custom)?;

        Ok(Http(role))
    }
}

impl Serialize for Http<asset::Bitcoin> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.as_sat().to_string())
    }
}

impl<'de> Deserialize<'de> for Http<asset::Bitcoin> {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let value =
            u64::from_str(value.as_str()).map_err(<D as Deserializer<'de>>::Error::custom)?;
        let amount = asset::Bitcoin::from_sat(value);

        Ok(Http(amount))
    }
}

impl Serialize for Http<transaction::Bitcoin> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.txid().to_string())
    }
}

impl Serialize for Http<transaction::Ethereum> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.hash.serialize(serializer)
    }
}

impl Serialize for Http<identity::Bitcoin> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let public_key = bitcoin::PublicKey::from(self.0);
        serializer.serialize_str(&public_key.to_string())
    }
}

impl Serialize for Http<Chain> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.0 {
            Chain::Bitcoin => serializer.serialize_str("bitcoin"),
        }
    }
}

impl_serialize_type_with_fields!(htlc_location::Bitcoin { "txid" => txid, "vout" => vout });
impl_serialize_http!(crate::ethereum::Address);
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

impl Serialize for Http<halight::Network> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let str = match self {
            Http(halight::Network::Mainnet) => "mainnet",
            Http(halight::Network::Testnet) => "testnet",
            Http(halight::Network::Regtest) => "regtest",
        };

        serializer.serialize_str(str)
    }
}

impl<'de> Deserialize<'de> for Http<halight::Network> {
    fn deserialize<D>(deserializer: D) -> Result<Http<halight::Network>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let network = match String::deserialize(deserializer)?.as_str() {
            "mainnet" => halight::Network::Mainnet,
            "testnet" => halight::Network::Testnet,
            "regtest" => halight::Network::Regtest,

            network => {
                return Err(<D as Deserializer<'de>>::Error::custom(format!(
                    "not regtest: {}",
                    network
                )))
            }
        };

        Ok(Http(network))
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

impl<'de> Deserialize<'de> for Http<bitcoin::Address> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let address = String::deserialize(deserializer)?;
        let address = bitcoin::Address::from_str(address.as_str())
            .map_err(<D as Deserializer<'de>>::Error::custom)?;

        Ok(Http(address))
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DialInformation {
    JustPeerId(Http<PeerId>),
    WithAddressHint {
        peer_id: Http<PeerId>,
        address_hint: Multiaddr,
    },
}

impl From<DialInformation> for PeerId {
    fn from(dial_information: DialInformation) -> Self {
        match dial_information {
            DialInformation::JustPeerId(inner) => inner.0,
            DialInformation::WithAddressHint { peer_id, .. } => peer_id.0,
        }
    }
}

impl From<DialInformation> for comit::network::DialInformation {
    fn from(dial_information: DialInformation) -> Self {
        match dial_information {
            DialInformation::JustPeerId(inner) => Self {
                peer_id: inner.0,
                address_hint: None,
            },
            DialInformation::WithAddressHint {
                peer_id,
                address_hint,
            } => Self {
                peer_id: peer_id.0,
                address_hint: Some(address_hint),
            },
        }
    }
}

/// An enum describing all the possible values of `alpha_ledger` and
/// `beta_ledger`.
///
/// Note: This enum makes use of serde's "try_from" and "into" feature: https://serde.rs/container-attrs.html#from
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
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
    Bitcoin(asset::Bitcoin),
    Ether(asset::Ether),
    Erc20(asset::Erc20),
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct EthereumLedgerParams {
    chain_id: Option<ChainId>,
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
    quantity: Http<asset::Bitcoin>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EtherAssetParams {
    quantity: asset::Ether,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Erc20AssetParams {
    quantity: asset::Erc20Quantity,
    token_contract: identity::Ethereum,
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
            HttpLedger::Ethereum(ledger) => HttpLedgerParams::Ethereum(ledger.into()),
            HttpLedger::Bitcoin(ledger) => HttpLedgerParams::Bitcoin(ledger.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("The Ethereum ledger requires either a network or a chain-id parameter.")]
pub struct InvalidEthereumLedgerParams;

impl TryFrom<EthereumLedgerParams> for ledger::Ethereum {
    type Error = InvalidEthereumLedgerParams;

    fn try_from(params: EthereumLedgerParams) -> Result<Self, Self::Error> {
        let chain_id = match params {
            EthereumLedgerParams {
                chain_id: Some(chain_id),
            } => chain_id,
            EthereumLedgerParams { chain_id: None } => return Err(InvalidEthereumLedgerParams),
        };

        Ok(Self { chain_id })
    }
}

impl From<ledger::Ethereum> for EthereumLedgerParams {
    fn from(ethereum: ledger::Ethereum) -> Self {
        let chain_id = ethereum.chain_id;

        Self {
            chain_id: Some(chain_id),
        }
    }
}

impl From<BitcoinLedgerParams> for ledger::Bitcoin {
    fn from(params: BitcoinLedgerParams) -> Self {
        params.network.0.into()
    }
}

impl From<ledger::Bitcoin> for BitcoinLedgerParams {
    fn from(bitcoin: ledger::Bitcoin) -> Self {
        Self {
            network: Http(bitcoin.into()),
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

impl From<BitcoinAssetParams> for asset::Bitcoin {
    fn from(params: BitcoinAssetParams) -> Self {
        *params.quantity
    }
}

impl From<asset::Bitcoin> for BitcoinAssetParams {
    fn from(bitcoin: asset::Bitcoin) -> Self {
        Self {
            quantity: Http(bitcoin),
        }
    }
}

impl From<EtherAssetParams> for asset::Ether {
    fn from(params: EtherAssetParams) -> Self {
        params.quantity
    }
}

impl From<asset::Ether> for EtherAssetParams {
    fn from(ether: asset::Ether) -> Self {
        Self { quantity: ether }
    }
}

impl From<Erc20AssetParams> for asset::Erc20 {
    fn from(params: Erc20AssetParams) -> Self {
        Self {
            token_contract: params.token_contract,
            quantity: params.quantity,
        }
    }
}

impl From<asset::Erc20> for Erc20AssetParams {
    fn from(erc20: asset::Erc20) -> Self {
        Self {
            quantity: erc20.quantity,
            token_contract: erc20.token_contract,
        }
    }
}

impl From<ledger::Bitcoin> for HttpLedger {
    fn from(ledger: ledger::Bitcoin) -> Self {
        HttpLedger::Bitcoin(ledger)
    }
}

impl From<ledger::Ethereum> for HttpLedger {
    fn from(ethereum: ledger::Ethereum) -> Self {
        HttpLedger::Ethereum(ethereum)
    }
}

impl From<asset::Bitcoin> for HttpAsset {
    fn from(bitcoin: asset::Bitcoin) -> Self {
        HttpAsset::Bitcoin(bitcoin)
    }
}

impl From<asset::Ether> for HttpAsset {
    fn from(ether: asset::Ether) -> Self {
        HttpAsset::Ether(ether)
    }
}

impl From<asset::Erc20> for HttpAsset {
    fn from(erc20: asset::Erc20) -> Self {
        HttpAsset::Erc20(erc20)
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("action not found")]
pub struct ActionNotFound;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset,
        asset::ethereum::FromWei,
        bitcoin::PublicKey,
        ethereum::{Address, ChainId, Hash, U256},
        http_api::{Http, HttpAsset, HttpLedger},
        swap_protocols::{
            ledger::{self},
            rfc003::SwapId,
            HashFunction, SwapProtocol,
        },
        transaction,
    };
    use ::bitcoin::{
        hashes::{hex::FromHex, sha256d},
        OutPoint, Script, TxIn,
    };
    use libp2p::PeerId;
    use std::str::FromStr;

    #[test]
    fn http_asset_serializes_correctly_to_json() {
        let bitcoin = HttpAsset::from(asset::Bitcoin::from_sat(100_000_000));
        let ether = HttpAsset::from(asset::Ether::from_wei(1_000_000_000_000_000_000u64)); // One exawei is one Ether
        let pay = HttpAsset::from(asset::Erc20::new(
            "B97048628DB6B661D4C2aA833e95Dbe1A905B280".parse().unwrap(),
            asset::Erc20Quantity::from_wei(U256::from(100_000_000_000u64)),
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
        assert_eq!(
            &pay_serialized,
            r#"{"name":"erc20","quantity":"100000000000","token_contract":"0xb97048628db6b661d4c2aa833e95dbe1a905b280"}"#
        );
    }

    #[test]
    fn bitcoin_http_ledger_regtest_serializes_correctly_to_json() {
        let input = &[
            HttpLedger::from(ledger::Bitcoin::Mainnet),
            HttpLedger::from(ledger::Bitcoin::Testnet),
            HttpLedger::from(ledger::Bitcoin::Regtest),
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
            HttpLedger::from(ledger::Ethereum::new(ChainId::from(1))),
            HttpLedger::from(ledger::Ethereum::new(ChainId::from(3))),
            HttpLedger::from(ledger::Ethereum::new(ChainId::from(1337))),
        ];

        let expected = &[
            r#"{"name":"ethereum","chain_id":1}"#,
            r#"{"name":"ethereum","chain_id":3}"#,
            r#"{"name":"ethereum","chain_id":1337}"#,
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
        let bitcoin_tx = transaction::Bitcoin {
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
        let ethereum_tx = transaction::Ethereum {
            hash: Hash::from([1u8; 32]),
            ..transaction::Ethereum::default()
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
        let bitcoin_identity: PublicKey =
            "02ef606e64a51b07373f81e042887e8e9c3806f0ff3fe3711df18beba8b82d82e6"
                .parse()
                .unwrap();

        let ethereum_identity = Address::from([7u8; 20]);

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
            .unwrap()
            .into(),
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

    #[test]
    fn http_halight_network_serializes_correctly_to_json() {
        let network = Http(halight::Network::Mainnet);
        let serialized = serde_json::to_string(&network).unwrap();
        assert_eq!(serialized, r#""mainnet""#);

        let network = Http(halight::Network::Testnet);
        let serialized = serde_json::to_string(&network).unwrap();
        assert_eq!(serialized, r#""testnet""#);

        let network = Http(halight::Network::Regtest);
        let serialized = serde_json::to_string(&network).unwrap();
        assert_eq!(serialized, r#""regtest""#);
    }
}
