use bitcoin_support::{serialize::serialize_hex, BitcoinQuantity};
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
        bitcoin,
        bob::PendingResponses,
        ethereum,
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

#[derive(Clone, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum GetActionQueryParams {
    BitcoinAddressAndFee {
        address: bitcoin_support::Address,
        fee_per_byte: String,
    },
    None {},
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum ActionResponseBody {
    SendToBitcoinAddress {
        address: bitcoin_support::Address,
        value: BitcoinQuantity,
    },
    BroadcastSignedBitcoinTransaction {
        hex: String,
    },
    SendEthereumTransaction {
        to: Option<ethereum_support::Address>,
        data: ethereum_support::Bytes,
        value: EtherQuantity,
        gas_limit: ethereum_support::U256,
    },
}

pub trait IntoResponseBody {
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem>;
}

impl IntoResponseBody for bitcoin::SendToAddress {
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match query_params {
            GetActionQueryParams::None {} => {
                let bitcoin::SendToAddress { address, value } = self.clone();
                Ok(ActionResponseBody::SendToBitcoinAddress { address, value })
            }
            _ => {
                error!("Unexpected GET parameters for a bitcoin::SendToAddress action type. Expected: none.");
                Err(HttpApiProblem::with_title_and_type_from_status(400)
                    .set_detail("This action does not take any query parameters"))
            }
        }
    }
}

#[derive(Serialize)]
struct MissingQueryParameter {
    data_type: &'static str,
    description: &'static str,
}

impl IntoResponseBody for bitcoin::SpendOutput {
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match query_params {
            GetActionQueryParams::BitcoinAddressAndFee {
                address,
                fee_per_byte,
            } => match fee_per_byte.parse::<f64>() {
                Ok(fee_per_byte) => {
                    let transaction = self.spend_to(address).sign_with_rate(fee_per_byte);
                    match serialize_hex(&transaction) {
                        Ok(hex) => {
                            Ok(ActionResponseBody::BroadcastSignedBitcoinTransaction { hex })
                        }
                        Err(e) => {
                            error!("Could not serialized signed Bitcoin transaction: {:?}", e);
                            Err(
                                HttpApiProblem::with_title_and_type_from_status(500).set_detail(
                                    "Issue encountered when serializing Bitcoin transaction",
                                ),
                            )
                        }
                    }
                }
                Err(_) => Err(HttpApiProblem::with_title_and_type_from_status(400)
                    .set_detail("fee-per-byte is not a valid float")),
            },
            _ => {
                error!("Unexpected GET parameters for a bitcoin::SpendOutput action type. Expected: address and fee-per-byte.");
                let mut problem = HttpApiProblem::with_title_and_type_from_status(400)
                    .set_detail("This action requires additional query parameters");
                problem
                    .set_value(
                        "address",
                        &MissingQueryParameter {
                            data_type: "string",
                            description: "The bitcoin address to where the funds should be sent",
                        },
                    )
                    .expect("invalid use of HttpApiProblem");
                problem
                    .set_value(
                        "fee_per_byte",
                        &MissingQueryParameter {
                            data_type: "float",
                            description:
                                "The fee-per-byte you want to pay for the redeem transaction in satoshis",
                        },
                    )
                    .expect("invalid use of HttpApiProblem");

                Err(problem)
            }
        }
    }
}

impl IntoResponseBody for ethereum::ContractDeploy {
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        let ethereum::ContractDeploy {
            data,
            value,
            gas_limit,
        } = self;
        match query_params {
            GetActionQueryParams::None {} => Ok(ActionResponseBody::SendEthereumTransaction {
                to: None,
                data,
                value,
                gas_limit,
            }),
            _ => {
                error!("Unexpected GET parameters for an ethereum::ContractDeploy action type. Expected: None.");
                Err(HttpApiProblem::with_title_and_type_from_status(400)
                    .set_detail("This action does not take any query parameters"))
            }
        }
    }
}

impl IntoResponseBody for ethereum::SendTransaction {
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        let ethereum::SendTransaction {
            to,
            data,
            value,
            gas_limit,
        } = self;
        match query_params {
            GetActionQueryParams::None {} => Ok(ActionResponseBody::SendEthereumTransaction {
                to: Some(to),
                data,
                value,
                gas_limit,
            }),
            _ => {
                error!("Unexpected GET parameters for an ethereum::SendTransaction action. Expected: None.");
                Err(HttpApiProblem::with_title_and_type_from_status(400)
                    .set_detail("This action does not take any query parameters"))
            }
        }
    }
}

impl IntoResponseBody for () {
    fn into_response_body(
        self,
        _: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        error!("IntoResponseBody should not be called for the unit type");
        Err(HttpApiProblem::with_title_and_type_from_status(500))
    }
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> IntoResponseBody
    for Action<Accept, Decline, Deploy, Fund, Redeem, Refund>
where
    Deploy: IntoResponseBody,
    Fund: IntoResponseBody,
    Redeem: IntoResponseBody,
    Refund: IntoResponseBody,
{
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match self {
            Action::Deploy(payload) => payload.into_response_body(query_params),
            Action::Fund(payload) => payload.into_response_body(query_params),
            Action::Redeem(payload) => payload.into_response_body(query_params),
            Action::Refund(payload) => payload.into_response_body(query_params),
            _ => {
                error!("IntoResponseBody is not implemented for Accept/Decline");
                Err(HttpApiProblem::with_title_and_type_from_status(500))
            }
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
    let metadata = metadata_store
        .get(&id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| match metadata {
            Metadata {
                role: RoleKind::Alice,
                ..
            } => Err(HttpApiProblem::with_title_and_type_from_status(404)),
            Metadata {
                role: RoleKind::Bob,
                ..
            } => match action {
                PostAction::Accept => {
                    serde_json::from_value::<AcceptSwapRequestHttpBody<AL, BL>>(body)
                        .map_err(|e| {
                            error!(
                                "Failed to deserialize body of accept response for swap {}: {:?}",
                                id, e
                            );
                            problem::serde(e)
                        })
                        .and_then(|accept_body| {
                            let keypair = key_store.get_transient_keypair(&id.into(), b"SUCCESS");
                            forward_response::<AL, BL>(
                                pending_responses.as_ref(),
                                &id,
                                Ok(StateMachineResponse {
                                    alpha_ledger_success_identity: keypair,
                                    beta_ledger_refund_identity: accept_body
                                        .beta_ledger_refund_identity,
                                    beta_ledger_lock_duration: accept_body
                                        .beta_ledger_lock_duration,
                                }),
                            )
                        })
                }
                PostAction::Decline => Err(problem::not_yet_implemented("Declining a swap")),
            },
        })
    )
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

#[derive(Debug, PartialEq)]
pub enum GetAction {
    Fund,
    Deploy,
    Redeem,
    Refund,
}

impl GetAction {
    fn matches<Accept, Decline, Deploy, Fund, Redeem, Refund>(
        &self,
        other: &Action<Accept, Decline, Deploy, Fund, Redeem, Refund>,
    ) -> bool {
        match other {
            Action::Deploy(_) => *self == GetAction::Deploy,
            Action::Fund(_) => *self == GetAction::Fund,
            Action::Redeem(_) => *self == GetAction::Redeem,
            Action::Refund(_) => *self == GetAction::Refund,
            _ => false,
        }
    }
}

impl FromStr for GetAction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match s {
            "fund" => Ok(GetAction::Fund),
            "redeem" => Ok(GetAction::Redeem),
            "refund" => Ok(GetAction::Refund),
            "deploy" => Ok(GetAction::Deploy),
            _ => Err(()),
        }
    }
}

pub fn get<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    action: GetAction,
    query_params: GetActionQueryParams,
) -> Result<impl Reply, Rejection> {
    handle_get(metadata_store, state_store, &id, &action, query_params)
        .map_err(HttpApiProblemStdError::from)
        .map_err(warp::reject::custom)
}

fn handle_get<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: &SwapId,
    action: &GetAction,
    query_params: GetActionQueryParams,
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

            state
                .actions()
                .iter()
                .find_map(|state_action| {
                    if action.matches(state_action) {
                        Some(
                            state_action
                                .clone()
                                .into_response_body(query_params.clone())
                                .map(|body| warp::reply::json(&body)),
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    Err(HttpApiProblem::with_title_and_type_from_status(400)
                        .set_detail("Requested action is not supported for this swap"))
                })
        })
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_urlencoded;

    #[test]
    fn given_no_query_parameters_deserialize_to_none() {
        let s = "";

        let res = serde_urlencoded::from_str::<GetActionQueryParams>(s);
        assert_eq!(res, Ok(GetActionQueryParams::None {}));
    }

    #[test]
    fn given_bitcoin_identity_and_fee_deserialize_to_ditto() {
        let s = "address=1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa&fee_per_byte=10.59";

        let res = serde_urlencoded::from_str::<GetActionQueryParams>(s);
        assert_eq!(
            res,
            Ok(GetActionQueryParams::BitcoinAddressAndFee {
                address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".parse().unwrap(),
                fee_per_byte: "10.59".to_string(),
            })
        );
    }
}
