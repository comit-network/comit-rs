use crate::{
    http_api::{self, asset::HttpAsset, ledger::HttpLedger, problem, PeerDetails},
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        rfc003::{self, alice::AliceSpawner, messages::ToRequest, Ledger, SecretSource},
        SwapId, Timestamp,
    },
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Token, EtherQuantity};
use http_api_problem::{HttpApiProblem, StatusCode as HttpStatusCode};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

pub fn handle_post_swap<A: AliceSpawner>(
    alice_spawner: &A,
    request_body_kind: SwapRequestBodyKind,
) -> Result<SwapCreated, HttpApiProblem> {
    let id = SwapId::default();

    match request_body_kind {
        SwapRequestBodyKind::BitcoinEthereumBitcoinQuantityErc20Token(body) => {
            alice_spawner.spawn(id, body.peer.clone(), Box::new(body))?
        }
        SwapRequestBodyKind::BitcoinEthereumBitcoinQuantityEtherQuantity(body) => {
            alice_spawner.spawn(id, body.peer.clone(), Box::new(body))?
        }
        SwapRequestBodyKind::EthereumBitcoinEtherQuantityBitcoinQuantity(body) => {
            alice_spawner.spawn(id, body.peer.clone(), Box::new(body))?
        }
        SwapRequestBodyKind::EthereumBitcoinErc20TokenBitcoinQuantity(body) => {
            alice_spawner.spawn(id, body.peer.clone(), Box::new(body))?
        }
        SwapRequestBodyKind::UnsupportedCombination(body) => {
            log::error!(
                "Swapping {:?} for {:?} from {:?} to {:?} is not supported",
                body.alpha_asset,
                body.beta_asset,
                body.alpha_ledger,
                body.beta_ledger
            );
            return Err(problem::unsupported());
        }
        SwapRequestBodyKind::MalformedRequest(body) => {
            log::error!(
                "Malformed request body: {}",
                serde_json::to_string(&body)
                    .expect("failed to serialize serde_json::Value as string ?!")
            );
            return Err(HttpApiProblem::with_title_and_type_from_status(
                HttpStatusCode::BAD_REQUEST,
            )
            .set_detail("The request body was malformed."));
        }
    };

    Ok(SwapCreated { id })
}

#[derive(Serialize, Debug)]
pub struct SwapCreated {
    pub id: SwapId,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum SwapRequestBodyKind {
    BitcoinEthereumBitcoinQuantityErc20Token(
        SwapRequestBody<Bitcoin, Ethereum, BitcoinQuantity, Erc20Token, OnlyRedeem<Ethereum>>,
    ),
    BitcoinEthereumBitcoinQuantityEtherQuantity(
        SwapRequestBody<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity, OnlyRedeem<Ethereum>>,
    ),
    EthereumBitcoinErc20TokenBitcoinQuantity(
        SwapRequestBody<Ethereum, Bitcoin, Erc20Token, BitcoinQuantity, OnlyRefund<Ethereum>>,
    ),
    EthereumBitcoinEtherQuantityBitcoinQuantity(
        SwapRequestBody<Ethereum, Bitcoin, EtherQuantity, BitcoinQuantity, OnlyRefund<Ethereum>>,
    ),
    // It is important that these two come last because untagged enums are tried in order
    UnsupportedCombination(Box<UnsupportedSwapRequestBody>),
    MalformedRequest(serde_json::Value),
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SwapRequestBody<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset, PartialIdentities> {
    #[serde(with = "http_api::asset::serde_asset")]
    alpha_asset: AA,
    #[serde(with = "http_api::asset::serde_asset")]
    beta_asset: BA,
    #[serde(with = "http_api::ledger::serde_ledger")]
    alpha_ledger: AL,
    #[serde(with = "http_api::ledger::serde_ledger")]
    beta_ledger: BL,
    alpha_expiry: Timestamp,
    beta_expiry: Timestamp,
    #[serde(flatten)]
    partial_identities: PartialIdentities,
    peer: PeerDetails,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct OnlyRedeem<L: Ledger> {
    pub beta_ledger_redeem_identity: L::Identity,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct OnlyRefund<L: Ledger> {
    pub alpha_ledger_refund_identity: L::Identity,
}

#[derive(Debug, Clone)]
pub struct Identities<AL: Ledger, BL: Ledger> {
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_redeem_identity: BL::Identity,
}

pub trait ToIdentities<AL: Ledger, BL: Ledger> {
    fn to_identities(&self, secret_source: &dyn SecretSource) -> Identities<AL, BL>;
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct UnsupportedSwapRequestBody {
    alpha_asset: HttpAsset,
    beta_asset: HttpAsset,
    alpha_ledger: HttpLedger,
    beta_ledger: HttpLedger,
    alpha_ledger_refund_identity: Option<String>,
    beta_ledger_redeem_identity: Option<String>,
    alpha_expiry: Timestamp,
    beta_expiry: Timestamp,
    #[serde(with = "http_api::serde_peer_id")]
    peer: PeerId,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset, I: ToIdentities<AL, BL>>
    ToRequest<AL, BL, AA, BA> for SwapRequestBody<AL, BL, AA, BA, I>
{
    fn to_request(
        &self,
        secret_source: &dyn SecretSource,
    ) -> rfc003::messages::Request<AL, BL, AA, BA> {
        let Identities {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
        } = self.partial_identities.to_identities(secret_source);
        rfc003::messages::Request {
            alpha_asset: self.alpha_asset,
            beta_asset: self.beta_asset,
            alpha_ledger: self.alpha_ledger,
            beta_ledger: self.beta_ledger,
            alpha_expiry: self.alpha_expiry,
            beta_expiry: self.beta_expiry,
            secret_hash: secret_source.secret().hash(),
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
        }
    }
}

impl ToIdentities<Bitcoin, Ethereum> for OnlyRedeem<Ethereum> {
    fn to_identities(&self, secret_source: &dyn SecretSource) -> Identities<Bitcoin, Ethereum> {
        Identities {
            alpha_ledger_refund_identity: secret_source.secp256k1_refund().into(),
            beta_ledger_redeem_identity: self.beta_ledger_redeem_identity,
        }
    }
}

impl ToIdentities<Ethereum, Bitcoin> for OnlyRefund<Ethereum> {
    fn to_identities(&self, secret_source: &dyn SecretSource) -> Identities<Ethereum, Bitcoin> {
        Identities {
            alpha_ledger_refund_identity: self.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: secret_source.secp256k1_redeem().into(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::http_api::{PeerDetails, PeerIdAndAddress};
    use spectral::prelude::*;

    #[test]
    fn can_deserialize_swap_request_body() {
        let body = r#"{
                "alpha_ledger": {
                    "name": "bitcoin",
                    "network": "regtest"
                },
                "beta_ledger": {
                    "name": "ethereum",
                    "network": "regtest"
                },
                "alpha_asset": {
                    "name": "bitcoin",
                    "quantity": "100000000"
                },
                "beta_asset": {
                    "name": "ether",
                    "quantity": "10000000000000000000"
                },
                "beta_ledger_redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                "alpha_expiry": 2000000000,
                "beta_expiry": 2000000000,
                "peer": "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi"
            }"#;

        let body = serde_json::from_str(body);

        assert_that(&body).is_ok_containing(SwapRequestBody {
            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_expiry: Timestamp::from(2000000000),
            beta_expiry: Timestamp::from(2000000000),
            partial_identities: OnlyRedeem::<Ethereum> {
                beta_ledger_redeem_identity: ethereum_support::Address::from(
                    "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                ),
            },
            peer: PeerDetails::PeerIdOnly(
                "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi"
                    .parse()
                    .unwrap(),
            ),
        })
    }

    #[test]
    fn give_peer_id_with_address_can_deserialize_swap_request_body() {
        let body = r#"{
                "alpha_ledger": {
                    "name": "bitcoin",
                    "network": "regtest"
                },
                "beta_ledger": {
                    "name": "ethereum",
                    "network": "regtest"
                },
                "alpha_asset": {
                    "name": "bitcoin",
                    "quantity": "100000000"
                },
                "beta_asset": {
                    "name": "ether",
                    "quantity": "10000000000000000000"
                },
                "beta_ledger_redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                "alpha_expiry": 2000000000,
                "beta_expiry": 2000000000,
                "peer": { "peer_id": "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi", "address": "/ip4/8.9.0.1/tcp/9999" }
            }"#;

        let body = serde_json::from_str(body);

        assert_that(&body).is_ok_containing(SwapRequestBody {
            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_expiry: Timestamp::from(2000000000),
            beta_expiry: Timestamp::from(2000000000),
            partial_identities: OnlyRedeem::<Ethereum> {
                beta_ledger_redeem_identity: ethereum_support::Address::from(
                    "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                ),
            },
            peer: PeerDetails::PeerIdAndAddress(PeerIdAndAddress {
                peer_id: "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi"
                    .parse()
                    .unwrap(),
                address: "/ip4/8.9.0.1/tcp/9999".parse().unwrap(),
            }),
        })
    }
}
