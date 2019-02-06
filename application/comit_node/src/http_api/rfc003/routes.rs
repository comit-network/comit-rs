use crate::{
    http_api::{
        self,
        asset::HttpAsset,
        ledger::HttpLedger,
        problem::HttpApiProblemStdError,
        rfc003::{
            handlers::{
                handle_get_action, handle_get_swap, handle_get_swaps, handle_post_action,
                handle_post_swap,
            },
            socket_addr,
        },
    },
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        rfc003::{alice::AliceSpawner, state_store::StateStore, Ledger, SecretSource, Timestamp},
        MetadataStore, SwapId,
    },
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Token, EtherQuantity};
use http_api_problem::HttpApiProblem;
use hyper::header;
use rustic_hal::HalResource;
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use warp::{Rejection, Reply};

pub const PROTOCOL_NAME: &str = "rfc003";
pub fn swap_path(id: SwapId) -> String {
    format!("/{}/{}/{}", http_api::PATH, PROTOCOL_NAME, id)
}

#[derive(Debug, Serialize)]
pub struct SwapDescription {
    pub alpha_ledger: HttpLedger,
    pub beta_ledger: HttpLedger,
    pub alpha_asset: HttpAsset,
    pub beta_asset: HttpAsset,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
}

#[derive(Debug, Serialize)]
pub struct GetSwapResource {
    pub swap: SwapDescription,
    pub role: String,
    pub state: String,
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
pub struct SwapRequestBody<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    #[serde(with = "http_api::asset::serde")]
    pub alpha_asset: AA,
    #[serde(with = "http_api::asset::serde")]
    pub beta_asset: BA,
    #[serde(with = "http_api::ledger::serde")]
    pub alpha_ledger: AL,
    #[serde(with = "http_api::ledger::serde")]
    pub beta_ledger: BL,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    #[serde(flatten)]
    pub identities: SwapRequestBodyIdentities<AL::Identity, BL::Identity>,
    #[serde(with = "socket_addr")]
    pub peer: SocketAddr,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct UnsupportedSwapRequestBody {
    pub alpha_asset: HttpAsset,
    pub beta_asset: HttpAsset,
    pub alpha_ledger: HttpLedger,
    pub beta_ledger: HttpLedger,
    pub alpha_ledger_refund_identity: Option<String>,
    pub beta_ledger_redeem_identity: Option<String>,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
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

#[allow(clippy::needless_pass_by_value)]
pub fn post_swap<A: AliceSpawner>(
    alice_spawner: A,
    secret_source: Arc<dyn SecretSource>,
    request_body_kind: SwapRequestBodyKind,
) -> Result<impl Reply, Rejection> {
    handle_post_swap(&alice_spawner, secret_source.as_ref(), request_body_kind)
        .map(|swap_created| {
            let body = warp::reply::json(&swap_created);
            let response =
                warp::reply::with_header(body, header::LOCATION, swap_path(swap_created.id));
            warp::reply::with_status(response, warp::http::StatusCode::CREATED)
        })
        .map_err(|problem| warp::reject::custom(HttpApiProblemStdError::from(problem)))
}

pub type ActionName = String;

#[allow(clippy::needless_pass_by_value)]
pub fn get_swap<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
) -> Result<impl Reply, Rejection> {
    let result: Result<(GetSwapResource, Vec<ActionName>), HttpApiProblem> =
        handle_get_swap(&metadata_store, &state_store, &id);

    match result {
        Ok((swap_resource, actions)) => {
            let mut response = HalResource::new(swap_resource);
            for action in actions {
                let route = format!("{}/{}", swap_path(id), action);
                response.with_link(action, route);
            }
            Ok(warp::reply::json(&response))
        }
        Err(e) => Err(warp::reject::custom(HttpApiProblemStdError::from(e))),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swaps<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
) -> Result<impl Reply, Rejection> {
    match handle_get_swaps(metadata_store.as_ref(), state_store.as_ref()) {
        Ok(swaps) => {
            let mut response = HalResource::new("");
            response.with_resources("swaps", swaps);
            Ok(warp::reply::json(&response))
        }
        Err(e) => Err(warp::reject::custom(HttpApiProblemStdError::from(e))),
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PostAction {
    Accept,
    Decline,
}

impl FromStr for PostAction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match s {
            "accept" => Ok(PostAction::Accept),
            "decline" => Ok(PostAction::Decline),
            _ => Err(()),
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn post_action<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    secret_source: Arc<dyn SecretSource>,
    id: SwapId,
    action: PostAction,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    handle_post_action(
        metadata_store.as_ref(),
        state_store.as_ref(),
        secret_source.as_ref(),
        id,
        action,
        body,
    )
    .map(|_| warp::reply())
    .map_err(HttpApiProblemStdError::from)
    .map_err(warp::reject::custom)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GetAction {
    Fund,
    Deploy,
    Redeem,
    Refund,
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum GetActionQueryParams {
    BitcoinAddressAndFee {
        address: bitcoin_support::Address,
        fee_per_byte: String,
    },
    None {},
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_action<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    action: GetAction,
    query_params: GetActionQueryParams,
) -> Result<impl Reply, Rejection> {
    handle_get_action(
        metadata_store.as_ref(),
        state_store,
        &id,
        action,
        &query_params,
    )
    .map_err(HttpApiProblemStdError::from)
    .map_err(warp::reject::custom)
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
