use crate::{
    http_api::{self, asset::HttpAsset, ledger::HttpLedger, problem},
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            self,
            alice::{AliceSpawner, SwapRequestIdentities},
            Ledger, SecretSource, Timestamp,
        },
        SwapId,
    },
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Token, EtherQuantity};
use http_api_problem::HttpApiProblem;
use std::net::SocketAddr;

pub fn handle_post_swap<A: AliceSpawner>(
    alice_spawner: &A,
    secret_source: &dyn SecretSource,
    request_body_kind: SwapRequestBodyKind,
) -> Result<SwapCreated, HttpApiProblem> {
    let id = SwapId::default();

    match request_body_kind {
        SwapRequestBodyKind::BitcoinEthereumBitcoinQuantityErc20Token(body) => alice_spawner
            .spawn(
                id,
                rfc003::alice::SwapRequest::from_swap_request_body(body, id, secret_source)?,
            )?,
        SwapRequestBodyKind::BitcoinEthereumBitcoinQuantityEtherQuantity(body) => alice_spawner
            .spawn(
                id,
                rfc003::alice::SwapRequest::from_swap_request_body(body, id, secret_source)?,
            )?,
        SwapRequestBodyKind::EthereumBitcoinEtherQuantityBitcoinQuantity(body) => alice_spawner
            .spawn(
                id,
                rfc003::alice::SwapRequest::from_swap_request_body(body, id, secret_source)?,
            )?,
        SwapRequestBodyKind::EthereumBitcoinErc20TokenBitcoinQuantity(body) => alice_spawner
            .spawn(
                id,
                rfc003::alice::SwapRequest::from_swap_request_body(body, id, secret_source)?,
            )?,
        SwapRequestBodyKind::UnsupportedCombination(body) => {
            error!(
                "Swapping {:?} for {:?} from {:?} to {:?} is not supported",
                body.alpha_asset, body.beta_asset, body.alpha_ledger, body.beta_ledger
            );
            return Err(problem::unsupported());
        }
        SwapRequestBodyKind::MalformedRequest(body) => {
            error!(
                "Malformed request body: {}",
                serde_json::to_string(&body)
                    .expect("failed to serialize serde_json::Value as string ?!")
            );
            return Err(HttpApiProblem::with_title_and_type_from_status(400)
                .set_detail("The request body was malformed"));
        }
    };

    Ok(SwapCreated { id })
}

#[derive(Serialize, Debug)]
pub struct SwapCreated {
    pub id: SwapId,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SwapRequestBody<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    #[serde(with = "http_api::asset::serde")]
    alpha_asset: AA,
    #[serde(with = "http_api::asset::serde")]
    beta_asset: BA,
    #[serde(with = "http_api::ledger::serde")]
    alpha_ledger: AL,
    #[serde(with = "http_api::ledger::serde")]
    beta_ledger: BL,
    alpha_expiry: Timestamp,
    beta_expiry: Timestamp,
    #[serde(flatten)]
    identities: SwapRequestBodyIdentities<AL::Identity, BL::Identity>,
    #[serde(with = "http_api::rfc003::socket_addr")]
    peer: SocketAddr,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SwapRequestBodyIdentities<AI, BI> {
    RefundAndRedeem {
        alpha_ledger_refund_identity: AI,
        beta_ledger_redeem_identity: BI,
    },
    OnlyRedeem {
        beta_ledger_redeem_identity: BI,
    },
    OnlyRefund {
        alpha_ledger_refund_identity: AI,
    },
    None {},
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
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SwapRequestBodyKind {
    BitcoinEthereumBitcoinQuantityErc20Token(
        SwapRequestBody<Bitcoin, Ethereum, BitcoinQuantity, Erc20Token>,
    ),
    BitcoinEthereumBitcoinQuantityEtherQuantity(
        SwapRequestBody<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>,
    ),
    EthereumBitcoinErc20TokenBitcoinQuantity(
        SwapRequestBody<Ethereum, Bitcoin, Erc20Token, BitcoinQuantity>,
    ),
    EthereumBitcoinEtherQuantityBitcoinQuantity(
        SwapRequestBody<Ethereum, Bitcoin, EtherQuantity, BitcoinQuantity>,
    ),
    // It is important that these two come last because untagged enums are tried in order
    UnsupportedCombination(Box<UnsupportedSwapRequestBody>),
    MalformedRequest(serde_json::Value),
}

trait FromSwapRequestBodyIdentities<AL: Ledger, BL: Ledger>
where
    Self: Sized,
{
    fn from_swap_request_body_identities(
        identities: SwapRequestBodyIdentities<AL::Identity, BL::Identity>,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem>;
}

impl FromSwapRequestBodyIdentities<Bitcoin, Ethereum>
    for rfc003::alice::SwapRequestIdentities<Bitcoin, Ethereum>
{
    fn from_swap_request_body_identities(
        identities: SwapRequestBodyIdentities<
            bitcoin_support::PubkeyHash,
            ethereum_support::Address,
        >,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem> {
        match identities {
            SwapRequestBodyIdentities::RefundAndRedeem { .. }
            | SwapRequestBodyIdentities::OnlyRefund { .. }
            | SwapRequestBodyIdentities::None {} => {
                Err(HttpApiProblem::with_title_and_type_from_status(400))
            }
            SwapRequestBodyIdentities::OnlyRedeem {
                beta_ledger_redeem_identity,
            } => Ok(rfc003::alice::SwapRequestIdentities {
                alpha_ledger_refund_identity: secret_source.new_secp256k1_refund(id),
                beta_ledger_redeem_identity,
            }),
        }
    }
}

impl FromSwapRequestBodyIdentities<Ethereum, Bitcoin>
    for rfc003::alice::SwapRequestIdentities<Ethereum, Bitcoin>
{
    fn from_swap_request_body_identities(
        identities: SwapRequestBodyIdentities<
            ethereum_support::Address,
            bitcoin_support::PubkeyHash,
        >,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem> {
        match identities {
            SwapRequestBodyIdentities::RefundAndRedeem { .. }
            | SwapRequestBodyIdentities::OnlyRedeem { .. }
            | SwapRequestBodyIdentities::None {} => {
                Err(HttpApiProblem::with_title_and_type_from_status(400))
            }
            SwapRequestBodyIdentities::OnlyRefund {
                alpha_ledger_refund_identity,
            } => Ok(rfc003::alice::SwapRequestIdentities {
                alpha_ledger_refund_identity,
                beta_ledger_redeem_identity: secret_source.new_secp256k1_redeem(id),
            }),
        }
    }
}

trait FromSwapRequestBody<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>
where
    Self: Sized,
{
    fn from_swap_request_body(
        body: SwapRequestBody<AL, BL, AA, BA>,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem>;
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> FromSwapRequestBody<AL, BL, AA, BA>
    for rfc003::alice::SwapRequest<AL, BL, AA, BA>
where
    SwapRequestIdentities<AL, BL>: FromSwapRequestBodyIdentities<AL, BL>,
{
    fn from_swap_request_body(
        body: SwapRequestBody<AL, BL, AA, BA>,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem> {
        Ok(rfc003::alice::SwapRequest {
            alpha_asset: body.alpha_asset,
            beta_asset: body.beta_asset,
            alpha_ledger: body.alpha_ledger,
            beta_ledger: body.beta_ledger,
            alpha_expiry: body.alpha_expiry,
            beta_expiry: body.beta_expiry,
            identities: SwapRequestIdentities::from_swap_request_body_identities(
                body.identities,
                id,
                secret_source,
            )?,
            bob_socket_address: body.peer,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use spectral::prelude::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn can_deserialize_swap_request_body_with_port() {
        let body = r#"{
                "alpha_ledger": {
                    "name": "Bitcoin",
                    "network": "regtest"
                },
                "beta_ledger": {
                    "name": "Ethereum",
                    "network": "regtest"
                },
                "alpha_asset": {
                    "name": "Bitcoin",
                    "quantity": "100000000"
                },
                "beta_asset": {
                    "name": "Ether",
                    "quantity": "10000000000000000000"
                },
                "alpha_ledger_refund_identity": null,
                "beta_ledger_redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                "alpha_expiry": 2000000000,
                "beta_expiry": 2000000000,
                "peer": "127.0.0.1:8002"
            }"#;

        let body = serde_json::from_str(body);

        assert_that(&body).is_ok_containing(SwapRequestBody {
            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_expiry: Timestamp::from(2000000000),
            beta_expiry: Timestamp::from(2000000000),
            identities: SwapRequestBodyIdentities::OnlyRedeem {
                beta_ledger_redeem_identity: ethereum_support::Address::from(
                    "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                ),
            },
            peer: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8002),
        })
    }

    #[test]
    fn can_deserialize_swap_request_body_without_port() {
        let body = r#"{
                "alpha_ledger": {
                    "name": "Bitcoin",
                    "network": "regtest"
                },
                "beta_ledger": {
                    "name": "Ethereum",
                    "network": "regtest"
                },
                "alpha_asset": {
                    "name": "Bitcoin",
                    "quantity": "100000000"
                },
                "beta_asset": {
                    "name": "Ether",
                    "quantity": "10000000000000000000"
                },
                "alpha_ledger_refund_identity": null,
                "beta_ledger_redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                "alpha_expiry": 2000000000,
                "beta_expiry": 2000000000,
                "peer": "127.0.0.1"
            }"#;

        let body = serde_json::from_str(body);

        assert_that(&body).is_ok_containing(SwapRequestBody {
            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_expiry: Timestamp::from(2000000000),
            beta_expiry: Timestamp::from(2000000000),
            identities: SwapRequestBodyIdentities::OnlyRedeem {
                beta_ledger_redeem_identity: ethereum_support::Address::from(
                    "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                ),
            },
            peer: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9939),
        })
    }

}
