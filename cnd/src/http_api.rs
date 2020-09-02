mod action;
mod amount;
mod dial_addr;
pub mod halbit;
mod halbit_herc20;
pub mod hbit;
mod hbit_herc20;
pub mod herc20;
mod herc20_halbit;
mod herc20_hbit;
mod info;
mod markets;
mod orders;
mod peers;
mod problem;
mod protocol;
mod route_factory;
mod serde_peer_id;
mod swaps;
mod tokens;

pub use self::{
    halbit::Halbit,
    hbit::Hbit,
    herc20::Herc20,
    problem::*,
    protocol::{AliceSwap, BobSwap},
    route_factory::create as create_routes,
};

pub const PATH: &str = "swaps";

use crate::{asset, ethereum::ChainId, identity, ledger, storage::CreatedSwap, LocalSwapId, Role};
use chrono::Utc;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};

/// Object representing the data of a POST request for creating a swap.
#[derive(Deserialize, Clone, Debug)]
pub struct PostBody<A, B> {
    pub alpha: A,
    pub beta: B,
    pub peer: DialInformation,
    pub role: Role,
}

impl<A, B> PostBody<A, B> {
    pub fn to_created_swap<CA, CB>(&self, swap_id: LocalSwapId) -> CreatedSwap<CA, CB>
    where
        CA: From<A>,
        CB: From<B>,
        Self: Clone,
    {
        let body = self.clone();

        let alpha = CA::from(body.alpha);
        let beta = CB::from(body.beta);

        let start_of_swap = Utc::now().naive_local();

        CreatedSwap {
            swap_id,
            alpha,
            beta,
            peer: body.peer.into(),
            address_hint: None,
            role: body.role,
            start_of_swap,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DialInformation {
    JustPeerId(#[serde(with = "serde_peer_id")] PeerId),
    WithAddressHint {
        #[serde(with = "serde_peer_id")]
        peer_id: PeerId,
        address_hint: Multiaddr,
    },
}

impl DialInformation {
    fn into_peer_with_address_hint(self) -> (PeerId, Option<Multiaddr>) {
        match self {
            DialInformation::JustPeerId(inner) => (inner, None),
            DialInformation::WithAddressHint {
                peer_id,
                address_hint,
            } => (peer_id, Some(address_hint)),
        }
    }
}

impl From<DialInformation> for PeerId {
    fn from(dial_information: DialInformation) -> Self {
        match dial_information {
            DialInformation::JustPeerId(inner) => inner,
            DialInformation::WithAddressHint { peer_id, .. } => peer_id,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct BitcoinLedgerParams {
    network: ledger::Bitcoin,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct BitcoinAssetParams {
    #[serde(with = "asset::bitcoin::sats_as_string")]
    quantity: asset::Bitcoin,
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
        params.network
    }
}

impl From<ledger::Bitcoin> for BitcoinLedgerParams {
    fn from(bitcoin: ledger::Bitcoin) -> Self {
        Self { network: bitcoin }
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
        params.quantity
    }
}

impl From<asset::Bitcoin> for BitcoinAssetParams {
    fn from(bitcoin: asset::Bitcoin) -> Self {
        Self { quantity: bitcoin }
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
    use crate::{
        asset,
        asset::ethereum::FromWei,
        ethereum::{ChainId, U256},
        http_api::{HttpAsset, HttpLedger},
        ledger,
    };

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
}
