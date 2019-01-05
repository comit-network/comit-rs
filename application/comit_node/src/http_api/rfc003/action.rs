use crate::{
    comit_client::SwapDeclineReason,
    http_api::{problem, HttpApiProblemStdError},
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        metadata_store::Metadata,
        rfc003::{
            alice, bitcoin,
            bob::{
                self,
                actions::{Accept, Decline},
            },
            ethereum,
            state_machine::StateMachineResponse,
            state_store::StateStore,
            Actions, Alice, Bob, Ledger, SecretSource,
        },
        AssetKind, LedgerKind, MetadataStore, RoleKind, SwapId,
    },
};
use bitcoin_support::{self, serialize::serialize_hex, BitcoinQuantity};
use ethereum_support::{self, Erc20Quantity, EtherQuantity};
use http_api_problem::HttpApiProblem;
use std::{str::FromStr, sync::Arc};
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

trait ExecuteAccept<AL: Ledger, BL: Ledger> {
    fn execute(
        &self,
        body: AcceptSwapRequestHttpBody<AL, BL>,
        secret_source: &dyn SecretSource,
        id: SwapId,
    ) -> Result<(), HttpApiProblem>;
}

impl<AL: Ledger, BL: Ledger> ExecuteAccept<AL, BL> for Accept<AL, BL>
where
    StateMachineResponse<AL::HtlcIdentity, BL::HtlcIdentity, BL::LockDuration>:
        FromAcceptSwapRequestHttpBody<AL, BL>,
{
    fn execute(
        &self,
        body: AcceptSwapRequestHttpBody<AL, BL>,
        secret_source: &dyn SecretSource,
        id: SwapId,
    ) -> Result<(), HttpApiProblem> {
        self.accept(StateMachineResponse::from_accept_swap_request_http_body(
            body,
            id,
            secret_source,
        )?)
        .map_err(|_| problem::action_already_taken())
    }
}

trait FromAcceptSwapRequestHttpBody<AL: Ledger, BL: Ledger>
where
    Self: Sized,
{
    fn from_accept_swap_request_http_body(
        body: AcceptSwapRequestHttpBody<AL, BL>,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem>;
}

impl FromAcceptSwapRequestHttpBody<Bitcoin, Ethereum>
    for StateMachineResponse<
        secp256k1_support::KeyPair,
        ethereum_support::Address,
        ethereum::Seconds,
    >
{
    fn from_accept_swap_request_http_body(
        body: AcceptSwapRequestHttpBody<Bitcoin, Ethereum>,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem> {
        match body {
            AcceptSwapRequestHttpBody::OnlyRedeem { .. } | AcceptSwapRequestHttpBody::RefundAndRedeem { .. } => Err(HttpApiProblem::with_title_and_type_from_status(400).set_detail("The redeem identity for swaps where Bitcoin is the AlphaLedger has to be provided on-demand, i.e. when the redeem action is executed.")),
            AcceptSwapRequestHttpBody::None { .. } => Err(HttpApiProblem::with_title_and_type_from_status(400).set_detail("Missing beta_ledger_refund_identity")),
            AcceptSwapRequestHttpBody::OnlyRefund { beta_ledger_refund_identity, beta_ledger_lock_duration } => Ok(StateMachineResponse {
                beta_ledger_refund_identity,
                beta_ledger_lock_duration,
                alpha_ledger_redeem_identity: secret_source.new_secp256k1_redeem(id),
            }),
        }
    }
}

impl FromAcceptSwapRequestHttpBody<Ethereum, Bitcoin>
    for StateMachineResponse<
        ethereum_support::Address,
        secp256k1_support::KeyPair,
        bitcoin_support::Blocks,
    >
{
    fn from_accept_swap_request_http_body(
        body: AcceptSwapRequestHttpBody<Ethereum, Bitcoin>,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem> {
        match body {
            AcceptSwapRequestHttpBody::OnlyRefund { .. } | AcceptSwapRequestHttpBody::RefundAndRedeem { .. } => Err(HttpApiProblem::with_title_and_type_from_status(400).set_detail("The refund identity for swaps where Bitcoin is the BetaLedger has to be provided on-demand, i.e. when the refund action is executed.")),
            AcceptSwapRequestHttpBody::None { .. } => Err(HttpApiProblem::with_title_and_type_from_status(400).set_detail("Missing beta_ledger_redeem_identity")),
            AcceptSwapRequestHttpBody::OnlyRedeem { alpha_ledger_redeem_identity, beta_ledger_lock_duration } => Ok(StateMachineResponse {
                alpha_ledger_redeem_identity,
                beta_ledger_lock_duration,
                beta_ledger_refund_identity: secret_source.new_secp256k1_refund(id),
            }),
        }
    }
}

trait ExecuteDecline {
    fn execute(&self, reason: Option<SwapDeclineReason>) -> Result<(), HttpApiProblem>;
}

impl<AL: Ledger, BL: Ledger> ExecuteDecline for Decline<AL, BL> {
    fn execute(&self, reason: Option<SwapDeclineReason>) -> Result<(), HttpApiProblem> {
        self.decline(reason)
            .map_err(|_| problem::action_already_taken())
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

impl<Deploy, Fund, Redeem, Refund> IntoResponseBody
    for alice::ActionKind<Deploy, Fund, Redeem, Refund>
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
            alice::ActionKind::Deploy(payload) => payload.into_response_body(query_params),
            alice::ActionKind::Fund(payload) => payload.into_response_body(query_params),
            alice::ActionKind::Redeem(payload) => payload.into_response_body(query_params),
            alice::ActionKind::Refund(payload) => payload.into_response_body(query_params),
        }
    }
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> IntoResponseBody
    for bob::ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>
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
            bob::ActionKind::Deploy(payload) => payload.into_response_body(query_params),
            bob::ActionKind::Fund(payload) => payload.into_response_body(query_params),
            bob::ActionKind::Redeem(payload) => payload.into_response_body(query_params),
            bob::ActionKind::Refund(payload) => payload.into_response_body(query_params),
            _ => {
                error!("IntoResponseBody is not implemented for Accept/Decline");
                Err(HttpApiProblem::with_title_and_type_from_status(500))
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
#[allow(dead_code)] // TODO: Remove once we have ledgers where we use all the combinations
enum AcceptSwapRequestHttpBody<AL: Ledger, BL: Ledger> {
    RefundAndRedeem {
        alpha_ledger_redeem_identity: AL::Identity,
        beta_ledger_refund_identity: BL::Identity,
        beta_ledger_lock_duration: BL::LockDuration,
    },
    OnlyRedeem {
        alpha_ledger_redeem_identity: AL::Identity,
        beta_ledger_lock_duration: BL::LockDuration,
    },
    OnlyRefund {
        beta_ledger_refund_identity: BL::Identity,
        beta_ledger_lock_duration: BL::LockDuration,
    },
    None {
        beta_ledger_lock_duration: BL::LockDuration,
    },
}

#[derive(Deserialize)]
struct DeclineSwapRequestHttpBody {
    reason: Option<SwapDeclineReason>,
}

#[allow(clippy::needless_pass_by_value)]
pub fn post<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    secret_source: Arc<dyn SecretSource>,
    id: SwapId,
    action: PostAction,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    handle_post(
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

#[allow(clippy::unit_arg, clippy::let_unit_value)]
pub fn handle_post<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: &T,
    state_store: &S,
    secret_source: &dyn SecretSource,
    id: SwapId,
    action: PostAction,
    body: serde_json::Value,
) -> Result<(), HttpApiProblem> {
    use crate::swap_protocols::{AssetKind, LedgerKind, Metadata, RoleKind};
    trace!("accept action requested on {:?}", id);
    let metadata = metadata_store
        .get(&id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types_bob!(
        &metadata,
        (|| match action {
            PostAction::Accept => serde_json::from_value::<AcceptSwapRequestHttpBody<AL, BL>>(body)
                .map_err(|e| {
                    error!(
                        "Failed to deserialize body of accept response for swap {}: {:?}",
                        id, e
                    );
                    problem::serde(&e)
                })
                .and_then(|accept_body| {
                    let state = state_store
                        .get::<Role>(&id)?
                        .ok_or_else(problem::state_store)?;

                    let accept_action = {
                        state
                            .actions()
                            .into_iter()
                            .find_map(move |action| match action {
                                bob::ActionKind::Accept(accept) => Some(Ok(accept)),
                                _ => None,
                            })
                            .unwrap_or_else(|| {
                                Err(HttpApiProblem::with_title_and_type_from_status(404))
                            })?
                    };

                    ExecuteAccept::execute(&accept_action, accept_body, secret_source, id)
                }),
            PostAction::Decline => {
                serde_json::from_value::<DeclineSwapRequestHttpBody>(body.clone())
                    .map_err(|e| {
                        error!(
                            "Failed to deserialize body of decline response for swap {}: {:?}",
                            id, e
                        );
                        problem::serde(&e)
                    })
                    .and_then(move |decline_body| {
                        let state = state_store
                            .get::<Role>(&id)?
                            .ok_or_else(problem::state_store)?;

                        let decline_action = {
                            state
                                .actions()
                                .into_iter()
                                .find_map(move |action| match action {
                                    bob::ActionKind::Decline(decline) => Some(Ok(decline)),
                                    _ => None,
                                })
                                .unwrap_or_else(|| {
                                    Err(HttpApiProblem::with_title_and_type_from_status(404))
                                })?
                        };

                        let reason = decline_body.reason;

                        ExecuteDecline::execute(&decline_action, reason)
                    })
            }
        })
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GetAction {
    Fund,
    Deploy,
    Redeem,
    Refund,
}

trait MatchesAction<A> {
    fn matches(self, action: &A) -> bool;
}

impl<Deploy, Fund, Redeem, Refund> MatchesAction<alice::ActionKind<Deploy, Fund, Redeem, Refund>>
    for GetAction
{
    fn matches(self, other: &alice::ActionKind<Deploy, Fund, Redeem, Refund>) -> bool {
        match other {
            alice::ActionKind::Deploy(_) => self == GetAction::Deploy,
            alice::ActionKind::Fund(_) => self == GetAction::Fund,
            alice::ActionKind::Redeem(_) => self == GetAction::Redeem,
            alice::ActionKind::Refund(_) => self == GetAction::Refund,
        }
    }
}
impl<Accept, Decline, Deploy, Fund, Redeem, Refund>
    MatchesAction<bob::ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>> for GetAction
{
    fn matches(
        self,
        other: &bob::ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>,
    ) -> bool {
        match other {
            bob::ActionKind::Deploy(_) => self == GetAction::Deploy,
            bob::ActionKind::Fund(_) => self == GetAction::Fund,
            bob::ActionKind::Redeem(_) => self == GetAction::Redeem,
            bob::ActionKind::Refund(_) => self == GetAction::Refund,
            _ => false,
        }
    }
}

impl FromStr for GetAction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match s {
            "deploy" => Ok(GetAction::Deploy),
            "fund" => Ok(GetAction::Fund),
            "redeem" => Ok(GetAction::Redeem),
            "refund" => Ok(GetAction::Refund),
            _ => Err(()),
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn get<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    action: GetAction,
    query_params: GetActionQueryParams,
) -> Result<impl Reply, Rejection> {
    handle_get(
        metadata_store.as_ref(),
        state_store,
        &id,
        action,
        &query_params,
    )
    .map_err(HttpApiProblemStdError::from)
    .map_err(warp::reject::custom)
}

fn handle_get<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: &T,
    state_store: Arc<S>,
    id: &SwapId,
    action: GetAction,
    query_params: &GetActionQueryParams,
) -> Result<impl Reply, HttpApiProblem> {
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

            state
                .actions()
                .iter()
                .find_map(|state_action| {
                    if action.matches(state_action) {
                        Some(
                            state_action
                                .clone()
                                .into_response_body(query_params.clone())
                                .map(|body| {
                                    trace!("Swap {}: Returning {:?} for {:?}", id, body, action);
                                    warp::reply::json(&body)
                                }),
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
