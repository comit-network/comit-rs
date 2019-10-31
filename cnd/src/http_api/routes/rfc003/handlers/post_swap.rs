use crate::{
    comit_client::Client,
    http_api::{self, asset::HttpAsset, ledger::HttpLedger, problem},
    network::DialInformation,
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            self,
            alice::{AliceSpawner, InitiateSwapRequest},
            messages::ToRequest,
            Ledger, SecretSource,
        },
        HashFunction, SwapId, Timestamp,
    },
};
use bitcoin::Amount as BitcoinAmount;
use ethereum_support::{Erc20Token, EtherQuantity};
use http_api_problem::{HttpApiProblem, StatusCode as HttpStatusCode};
use serde::{Deserialize, Serialize};

pub fn handle_post_swap<A: AliceSpawner + InitiateSwapRequest + Client>(
    alice: &A,
    request_body_kind: SwapRequestBodyKind,
) -> Result<SwapCreated, HttpApiProblem> {
    let id = SwapId::default();

    match request_body_kind {
        SwapRequestBodyKind::BitcoinEthereumBitcoinErc20Token(body) => {
            alice.initiate_swap_request(id, body.peer.clone(), Box::new(body.clone()))?;
            Ok(SwapCreated { id })
        }
        SwapRequestBodyKind::BitcoinEthereumBitcoinAmountEtherQuantity(body) => {
            alice.initiate_swap_request(id, body.peer.clone(), Box::new(body.clone()))?;
            Ok(SwapCreated { id })
        }
        SwapRequestBodyKind::EthereumBitcoinEtherQuantityBitcoinAmount(body) => {
            alice.initiate_swap_request(id, body.peer.clone(), Box::new(body.clone()))?;
            Ok(SwapCreated { id })
        }
        SwapRequestBodyKind::EthereumBitcoinErc20TokenBitcoinAmount(body) => {
            alice.initiate_swap_request(id, body.peer.clone(), Box::new(body.clone()))?;
            Ok(SwapCreated { id })
        }
        SwapRequestBodyKind::UnsupportedCombination(body) => {
            log::error!(
                "Swapping {:?} for {:?} from {:?} to {:?} is not supported",
                body.alpha_asset,
                body.beta_asset,
                body.alpha_ledger,
                body.beta_ledger
            );
            Err(problem::unsupported())
        }
        SwapRequestBodyKind::MalformedRequest(body) => {
            log::error!(
                "Malformed request body: {}",
                serde_json::to_string(&body)
                    .expect("failed to serialize serde_json::Value as string ?!")
            );
            Err(
                HttpApiProblem::with_title_and_type_from_status(HttpStatusCode::BAD_REQUEST)
                    .set_detail("The request body was malformed."),
            )
        }
    }
}

#[derive(Serialize, Debug)]
pub struct SwapCreated {
    pub id: SwapId,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum SwapRequestBodyKind {
    BitcoinEthereumBitcoinErc20Token(
        SwapRequestBody<Bitcoin, Ethereum, BitcoinAmount, Erc20Token, OnlyRedeem<Ethereum>>,
    ),
    BitcoinEthereumBitcoinAmountEtherQuantity(
        SwapRequestBody<Bitcoin, Ethereum, BitcoinAmount, EtherQuantity, OnlyRedeem<Ethereum>>,
    ),
    EthereumBitcoinErc20TokenBitcoinAmount(
        SwapRequestBody<Ethereum, Bitcoin, Erc20Token, BitcoinAmount, OnlyRefund<Ethereum>>,
    ),
    EthereumBitcoinEtherQuantityBitcoinAmount(
        SwapRequestBody<Ethereum, Bitcoin, EtherQuantity, BitcoinAmount, OnlyRefund<Ethereum>>,
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
    peer: DialInformation,
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
    peer: DialInformation,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset, I: ToIdentities<AL, BL>>
    ToRequest<AL, BL, AA, BA> for SwapRequestBody<AL, BL, AA, BA, I>
{
    fn to_request(
        &self,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> rfc003::Request<AL, BL, AA, BA> {
        let Identities {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
        } = self.partial_identities.to_identities(secret_source);
        rfc003::Request {
            id,
            alpha_asset: self.alpha_asset,
            beta_asset: self.beta_asset,
            alpha_ledger: self.alpha_ledger,
            beta_ledger: self.beta_ledger,
            hash_function: HashFunction::Sha256,
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
        let alpha_ledger_refund_identity = crate::bitcoin::PublicKey::from_secret_key(
            &*crate::SECP,
            &secret_source.secp256k1_refund(),
        );

        Identities {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: self.beta_ledger_redeem_identity,
        }
    }
}

impl ToIdentities<Ethereum, Bitcoin> for OnlyRefund<Ethereum> {
    fn to_identities(&self, secret_source: &dyn SecretSource) -> Identities<Ethereum, Bitcoin> {
        let beta_ledger_redeem_identity = crate::bitcoin::PublicKey::from_secret_key(
            &*crate::SECP,
            &secret_source.secp256k1_redeem(),
        );

        Identities {
            alpha_ledger_refund_identity: self.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{network::DialInformation, swap_protocols::ledger::ethereum::ChainId};
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
            alpha_asset: BitcoinAmount::from_btc(1.0).unwrap(),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_expiry: Timestamp::from(2_000_000_000),
            beta_expiry: Timestamp::from(2_000_000_000),
            partial_identities: OnlyRedeem::<Ethereum> {
                beta_ledger_redeem_identity: "00a329c0648769a73afac7f9381e08fb43dbea72"
                    .parse()
                    .unwrap(),
            },
            peer: DialInformation {
                peer_id: "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi"
                    .parse()
                    .unwrap(),
                address_hint: None,
            },
        })
    }

    #[test]
    fn given_peer_id_with_address_can_deserialize_swap_request_body() {
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
                "peer": { "peer_id": "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi", "address_hint": "/ip4/8.9.0.1/tcp/9999" }
            }"#;

        let body = serde_json::from_str(body);

        assert_that(&body).is_ok_containing(SwapRequestBody {
            alpha_asset: BitcoinAmount::from_btc(1.0).unwrap(),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_expiry: Timestamp::from(2_000_000_000),
            beta_expiry: Timestamp::from(2_000_000_000),
            partial_identities: OnlyRedeem::<Ethereum> {
                beta_ledger_redeem_identity: "00a329c0648769a73afac7f9381e08fb43dbea72"
                    .parse()
                    .unwrap(),
            },
            peer: DialInformation {
                peer_id: "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi"
                    .parse()
                    .unwrap(),
                address_hint: Some("/ip4/8.9.0.1/tcp/9999".parse().unwrap()),
            },
        })
    }

    #[test]
    fn can_deserialize_swap_request_body_with_chain_id() {
        let body = r#"{
                "alpha_ledger": {
                    "name": "bitcoin",
                    "network": "regtest"
                },
                "beta_ledger": {
                    "name": "ethereum",
                    "chain_id": 3
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
            alpha_asset: BitcoinAmount::from_btc(1.0).unwrap(),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::new(ChainId::new(3)),
            alpha_expiry: Timestamp::from(2_000_000_000),
            beta_expiry: Timestamp::from(2_000_000_000),
            partial_identities: OnlyRedeem::<Ethereum> {
                beta_ledger_redeem_identity: "00a329c0648769a73afac7f9381e08fb43dbea72"
                    .parse()
                    .unwrap(),
            },
            peer: DialInformation {
                peer_id: "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi"
                    .parse()
                    .unwrap(),
                address_hint: None,
            },
        })
    }
}
