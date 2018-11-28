use bitcoin_support::BitcoinQuantity;
use comit_client::SwapReject;
use ethereum_support::{Erc20Quantity, EtherQuantity};
use http_api::{problem, HttpApiProblemStdError};
use http_api_problem::HttpApiProblem;
use key_store::KeyStore;
use std::{str::FromStr, sync::Arc};
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    metadata_store::Metadata,
    rfc003::{
        actions::{Action, StateActions},
        bob::PendingResponses,
        roles::{Alice, Bob},
        state_machine::StateMachineResponse,
        state_store::StateStore,
        Ledger,
    },
    AssetKind, LedgerKind, MetadataStore, RoleKind,
};
use swaps::common::SwapId;
use warp::{self, Rejection, Reply};

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

#[derive(Debug, Deserialize, LabelledGeneric)]
struct AcceptSwapRequestHttpBody<AL: Ledger, BL: Ledger> {
    alpha_ledger_success_identity: AL::HttpIdentity,
    beta_ledger_refund_identity: BL::HttpIdentity,
    beta_ledger_lock_duration: BL::LockDuration,
}

pub fn post<T: MetadataStore<SwapId>>(
    metadata_store: Arc<T>,
    pending_responses: Arc<PendingResponses<SwapId>>,
    key_store: Arc<KeyStore>,
    id: SwapId,
    action: PostAction,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    handle_post(
        metadata_store,
        pending_responses,
        key_store,
        id,
        action,
        body,
    )
    .map(|_| warp::reply())
    .map_err(HttpApiProblemStdError::from)
    .map_err(warp::reject::custom)
}

pub fn handle_post<T: MetadataStore<SwapId>>(
    metadata_store: Arc<T>,
    pending_responses: Arc<PendingResponses<SwapId>>,
    key_store: Arc<KeyStore>,
    id: SwapId,
    action: PostAction,
    body: serde_json::Value,
) -> Result<(), HttpApiProblem> {
    use swap_protocols::{AssetKind, LedgerKind, Metadata, RoleKind};
    trace!("accept action requested on {:?}", id);
    match metadata_store.get(&id)? {
        Some(Metadata {
            alpha_ledger: LedgerKind::Bitcoin,
            beta_ledger: LedgerKind::Ethereum,
            alpha_asset: AssetKind::Bitcoin,
            beta_asset: AssetKind::Ether,
            role,
        }) => match role {
            RoleKind::Alice => Err(HttpApiProblem::with_title_and_type_from_status(400)
                .set_detail(format!("Swap {} was initiated by this comit_node, only the counter-part can accept or decline", id))),
            RoleKind::Bob => {
                match action {
                    PostAction::Accept => serde_json::from_value::<AcceptSwapRequestHttpBody<Bitcoin, Ethereum>>(body)
                        .map_err(|e| {
                            error!(
                                "Failed to deserialize body of accept response for swap {}: {:?}",
                                id, e
                            );
                            HttpApiProblem::new("invalid-body")
                                .set_status(400)
                                .set_detail("Failed to deserialize given body.")
                        })
                        .and_then(|accept_body| {
                            let keypair = key_store.get_transient_keypair(&id.into(), b"SUCCESS");
                            forward_response::<Bitcoin, Ethereum>(pending_responses.as_ref(), &id, Ok(StateMachineResponse{
                                alpha_ledger_success_identity: keypair,
                                beta_ledger_refund_identity: accept_body.beta_ledger_refund_identity,
                                beta_ledger_lock_duration: accept_body.beta_ledger_lock_duration,
                            }))
                        }),
                    PostAction::Decline => Err(HttpApiProblem::with_title_from_status(500)
                                           .set_detail("declining a swap is not yet implemented")),
                }
            }
        },
        Some(_) => Err(problem::unsupported()),
        None => {
            debug!("Metadata for {} not found", id);
            Err(HttpApiProblem::new("swap-not-found").set_status(404))
        }
    }
}

fn forward_response<AL: Ledger, BL: Ledger>(
    pending_responses: &PendingResponses<SwapId>,
    id: &SwapId,
    response: Result<
        StateMachineResponse<AL::HtlcIdentity, BL::HtlcIdentity, BL::LockDuration>,
        SwapReject,
    >,
) -> Result<(), HttpApiProblem> {
    pending_responses
        .take::<AL, BL>(id)
        .ok_or_else(|| HttpApiProblem::with_title_from_status(500))
        .and_then(|pending_response| {
            pending_response.send(response).map_err(|_| {
                error!(
                    "Failed to send pending response of swap {} through channel",
                    id
                );
                HttpApiProblem::with_title_from_status(500)
            })
        })
}

#[derive(Debug)]
pub enum GetAction {
    Fund,
    Redeem,
    Refund,
}

impl FromStr for GetAction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match s {
            "fund" => Ok(GetAction::Fund),
            "redeem" => Ok(GetAction::Redeem),
            "refund" => Ok(GetAction::Refund),
            _ => Err(()),
        }
    }
}

pub fn get<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    action: GetAction,
) -> Result<impl Reply, Rejection> {
    handle_get(metadata_store, state_store, &id, &action)
        .map_err(HttpApiProblemStdError::from)
        .map_err(warp::reject::custom)
}

fn handle_get<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: &SwapId,
    action: &GetAction,
) -> Result<impl Reply, HttpApiProblem> {
    let metadata = metadata_store
        .get(id)?
        .ok_or_else(problem::swap_not_found)?;
    get_swap!(
        &metadata,
        state_store,
        id,
        state,
        (|| {
            let state = state.ok_or(HttpApiProblem::with_title_and_type_from_status(500))?;
            trace!("Retrieved state for {}: {:?}", id, state);

            match action {
                GetAction::Fund => {
                    let action =
                        state
                            .actions()
                            .iter()
                            .find_map(|state_action| match state_action {
                                Action::Fund(fund_action) => {
                                    Some(serde_json::to_value(&fund_action).unwrap())
                                }
                                _ => None,
                            });

                    action.map(|action| warp::reply::json(&action)).ok_or(
                        HttpApiProblem::with_title_and_type_from_status(400)
                            .set_detail("Requested action is not supported for this swap"),
                    )
                }
                GetAction::Redeem => unimplemented!(),
                GetAction::Refund => unimplemented!(),
            }
        })
    )
}
