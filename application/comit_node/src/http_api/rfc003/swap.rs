use crate::{
    http_api::{
        self,
        asset::{HttpAsset, ToHttpAsset},
        ledger::{HttpLedger, ToHttpLedger},
        lock_duration::{HttpLockDuration, ToHttpLockDuration},
        problem::{self, HttpApiProblemStdError},
        rfc003::socket_addr,
    },
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            self,
            alice::{AliceSpawner, SwapRequestIdentities},
            state_store::StateStore,
            Actions, Alice, Bob, Ledger, SecretSource,
        },
        Metadata, MetadataStore, RoleKind, SwapId,
    },
};
use bitcoin_support::{self, BitcoinQuantity};
use ethereum_support::{self, Erc20Quantity, EtherQuantity};
use http_api_problem::HttpApiProblem;
use hyper::header;
use rustic_hal::HalResource;
use std::{net::SocketAddr, sync::Arc};
use warp::{self, Rejection, Reply};

pub const PROTOCOL_NAME: &str = "rfc003";

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SwapRequestBodyKind {
    BitcoinEthereumBitcoinQuantityErc20Quantity(
        SwapRequestBody<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>,
    ),
    BitcoinEthereumBitcoinQuantityEtherQuantity(
        SwapRequestBody<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>,
    ),
    EthereumBitcoinErc20QuantityBitcoinQuantity(
        SwapRequestBody<Ethereum, Bitcoin, Erc20Quantity, BitcoinQuantity>,
    ),
    EthereumBitcoinEtherQuantityBitcoinQuantity(
        SwapRequestBody<Ethereum, Bitcoin, EtherQuantity, BitcoinQuantity>,
    ),
    // It is important that these two come last because untagged enums are tried in order
    UnsupportedCombination(Box<UnsupportedSwapRequestBody>),
    MalformedRequest(serde_json::Value),
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
    alpha_ledger_lock_duration: AL::LockDuration,
    #[serde(flatten)]
    identities: SwapRequestBodyIdentities<AL::Identity, BL::Identity>,
    #[serde(with = "socket_addr")]
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
            alpha_ledger_lock_duration: body.alpha_ledger_lock_duration,
            identities: SwapRequestIdentities::from_swap_request_body_identities(
                body.identities,
                id,
                secret_source,
            )?,
            bob_socket_address: body.peer,
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct UnsupportedSwapRequestBody {
    alpha_asset: HttpAsset,
    beta_asset: HttpAsset,
    alpha_ledger: HttpLedger,
    beta_ledger: HttpLedger,
    alpha_ledger_refund_identity: Option<String>,
    beta_ledger_redeem_identity: Option<String>,
    alpha_ledger_lock_duration: i64,
}

#[derive(Serialize, Debug)]
pub struct SwapCreated {
    pub id: SwapId,
}

fn swap_path(id: SwapId) -> String {
    format!("/{}/{}/{}", http_api::PATH, PROTOCOL_NAME, id)
}

#[allow(clippy::needless_pass_by_value)]

pub fn post_swap<A: AliceSpawner>(
    alice_spawner: Arc<A>,
    secret_source: Arc<dyn SecretSource>,
    request_body_kind: SwapRequestBodyKind,
) -> Result<impl Reply, Rejection> {
    handle_post_swap(
        alice_spawner.as_ref(),
        secret_source.as_ref(),
        request_body_kind,
    )
    .map(|swap_created| {
        let body = warp::reply::json(&swap_created);
        let response = warp::reply::with_header(body, header::LOCATION, swap_path(swap_created.id));
        warp::reply::with_status(response, warp::http::StatusCode::CREATED)
    })
    .map_err(|problem| warp::reject::custom(HttpApiProblemStdError::from(problem)))
}

fn handle_post_swap<A: AliceSpawner>(
    alice_spawner: &A,
    secret_source: &dyn SecretSource,
    request_body_kind: SwapRequestBodyKind,
) -> Result<SwapCreated, HttpApiProblem> {
    let id = SwapId::default();

    match request_body_kind {
        SwapRequestBodyKind::BitcoinEthereumBitcoinQuantityErc20Quantity(body) => alice_spawner
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
        SwapRequestBodyKind::EthereumBitcoinErc20QuantityBitcoinQuantity(body) => alice_spawner
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

#[derive(Debug, Serialize)]
pub struct SwapDescription {
    alpha_ledger: HttpLedger,
    beta_ledger: HttpLedger,
    alpha_asset: HttpAsset,
    beta_asset: HttpAsset,
    alpha_lock_duration: HttpLockDuration,
    #[serde(skip_serializing_if = "Option::is_none")]
    beta_lock_duration: Option<HttpLockDuration>,
}

#[derive(Debug, Serialize)]
struct GetSwapResource {
    pub swap: SwapDescription,
    pub role: String,
    pub state: String,
}

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
        Err(e) => Err(warp::reject::custom(HttpApiProblemStdError::new(e))),
    }
}

type ActionName = String;

fn handle_get_swap<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: &Arc<T>,
    state_store: &Arc<S>,
    id: &SwapId,
) -> Result<(GetSwapResource, Vec<ActionName>), HttpApiProblem> {
    let metadata = metadata_store
        .get(id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<Role>(id)?
                .ok_or_else(problem::state_store)?;
            trace!("Retrieved state for {}: {:?}", id, state);

            let start_state = state.start_state()?;

            let actions: Vec<ActionName> =
                state.actions().iter().map(|action| action.name()).collect();
            (Ok((
                GetSwapResource {
                    state: state.name(),
                    swap: SwapDescription {
                        alpha_ledger: start_state.alpha_ledger.to_http_ledger().unwrap(),
                        beta_ledger: start_state.beta_ledger.to_http_ledger().unwrap(),
                        alpha_asset: start_state.alpha_asset.to_http_asset().unwrap(),
                        beta_asset: start_state.beta_asset.to_http_asset().unwrap(),
                        alpha_lock_duration: start_state
                            .alpha_ledger_lock_duration
                            .to_http_lock_duration()
                            .unwrap(),
                        beta_lock_duration: state
                            .beta_ledger_lock_duration()
                            .map(|lock| lock.to_http_lock_duration().unwrap()),
                    },
                    role: format!("{}", metadata.role),
                },
                actions,
            )))
        })
    )
}

#[derive(Serialize, Debug)]
pub struct EmbeddedSwapResource {
    state: String,
    protocol: String,
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
        Err(e) => {
            // FIXME: Log all 500 errors with a warp filter. see #584
            error!("returning error: {:?}", e);
            Err(warp::reject::custom(HttpApiProblemStdError::new(e)))
        }
    }
}

fn handle_get_swaps<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: &T,
    state_store: &S,
) -> Result<Vec<HalResource>, HttpApiProblem> {
    let mut resources = vec![];
    for (id, metadata) in metadata_store.all()?.into_iter() {
        with_swap_types!(
            &metadata,
            (|| -> Result<(), HttpApiProblem> {
                let state = state_store.get::<Role>(&id)?;

                match state {
                    Some(state) => {
                        let swap = EmbeddedSwapResource {
                            state: state.name(),
                            protocol: PROTOCOL_NAME.into(),
                        };

                        let mut hal_resource = HalResource::new(swap);
                        hal_resource.with_link("self", swap_path(id));
                        resources.push(hal_resource);
                    }
                    None => error!("Couldn't find state for {} despite having the metadata", id),
                };
                Ok(())
            })
        )?;
    }

    Ok(resources)
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
                "alpha_ledger_lock_duration": 144,
                "peer": "127.0.0.1:8002"
            }"#;

        let body = serde_json::from_str(body);

        assert_that(&body).is_ok_containing(SwapRequestBody {
            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_ledger_lock_duration: bitcoin_support::Blocks::new(144),
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
                "alpha_ledger_lock_duration": 144,
                "peer": "127.0.0.1"
            }"#;

        let body = serde_json::from_str(body);

        assert_that(&body).is_ok_containing(SwapRequestBody {
            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_ledger_lock_duration: bitcoin_support::Blocks::new(144),
            identities: SwapRequestBodyIdentities::OnlyRedeem {
                beta_ledger_redeem_identity: ethereum_support::Address::from(
                    "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                ),
            },
            peer: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9939),
        })
    }

}
