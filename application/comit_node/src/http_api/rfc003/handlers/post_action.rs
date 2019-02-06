use crate::{
    comit_client::SwapDeclineReason,
    http_api::{problem, rfc003::routes::PostAction},
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            bob::{
                self,
                actions::{Accept, Decline},
            },
            state_machine::StateMachineResponse,
            state_store::StateStore,
            Actions, Bob, Ledger, SecretSource,
        },
        MetadataStore, SwapId,
    },
};
use bitcoin_support;
use ethereum_support::{self, Erc20Token};
use http_api_problem::HttpApiProblem;

trait ExecuteAccept<AL: Ledger, BL: Ledger> {
    fn execute(
        &self,
        body: AcceptSwapRequestHttpBody<AL, BL>,
        secret_source: &dyn SecretSource,
        id: SwapId,
    ) -> Result<(), HttpApiProblem>;
}

#[allow(clippy::unit_arg, clippy::let_unit_value)]
pub fn handle_post_action<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: &T,
    state_store: &S,
    secret_source: &dyn SecretSource,
    id: SwapId,
    action: PostAction,
    body: serde_json::Value,
) -> Result<(), HttpApiProblem> {
    use crate::swap_protocols::{Metadata, RoleKind};
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

impl<AL: Ledger, BL: Ledger> ExecuteAccept<AL, BL> for Accept<AL, BL>
where
    StateMachineResponse<AL::HtlcIdentity, BL::HtlcIdentity>: FromAcceptSwapRequestHttpBody<AL, BL>,
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

trait ExecuteDecline {
    fn execute(&self, reason: Option<SwapDeclineReason>) -> Result<(), HttpApiProblem>;
}

impl<AL: Ledger, BL: Ledger> ExecuteDecline for Decline<AL, BL> {
    fn execute(&self, reason: Option<SwapDeclineReason>) -> Result<(), HttpApiProblem> {
        self.decline(reason)
            .map_err(|_| problem::action_already_taken())
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
#[allow(dead_code)] // TODO: Remove once we have ledgers where we use all the combinations
enum AcceptSwapRequestHttpBody<AL: Ledger, BL: Ledger> {
    RefundAndRedeem {
        alpha_ledger_redeem_identity: AL::Identity,
        beta_ledger_refund_identity: BL::Identity,
    },
    OnlyRedeem {
        alpha_ledger_redeem_identity: AL::Identity,
    },
    OnlyRefund {
        beta_ledger_refund_identity: BL::Identity,
    },
    None {},
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
    for StateMachineResponse<secp256k1_support::KeyPair, ethereum_support::Address>
{
    fn from_accept_swap_request_http_body(
        body: AcceptSwapRequestHttpBody<Bitcoin, Ethereum>,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem> {
        match body {
            AcceptSwapRequestHttpBody::OnlyRedeem { .. } | AcceptSwapRequestHttpBody::RefundAndRedeem { .. } => Err(HttpApiProblem::with_title_and_type_from_status(400).set_detail("The redeem identity for swaps where Bitcoin is the AlphaLedger has to be provided on-demand, i.e. when the redeem action is executed.")),
            AcceptSwapRequestHttpBody::None { .. } => Err(HttpApiProblem::with_title_and_type_from_status(400).set_detail("Missing beta_ledger_refund_identity")),
            AcceptSwapRequestHttpBody::OnlyRefund { beta_ledger_refund_identity } => Ok(StateMachineResponse {
                beta_ledger_refund_identity,
                alpha_ledger_redeem_identity: secret_source.new_secp256k1_redeem(id),
            }),
        }
    }
}

impl FromAcceptSwapRequestHttpBody<Ethereum, Bitcoin>
    for StateMachineResponse<ethereum_support::Address, secp256k1_support::KeyPair>
{
    fn from_accept_swap_request_http_body(
        body: AcceptSwapRequestHttpBody<Ethereum, Bitcoin>,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem> {
        match body {
            AcceptSwapRequestHttpBody::OnlyRefund { .. } | AcceptSwapRequestHttpBody::RefundAndRedeem { .. } => Err(HttpApiProblem::with_title_and_type_from_status(400).set_detail("The refund identity for swaps where Bitcoin is the BetaLedger has to be provided on-demand, i.e. when the refund action is executed.")),
            AcceptSwapRequestHttpBody::None { .. } => Err(HttpApiProblem::with_title_and_type_from_status(400).set_detail("Missing beta_ledger_redeem_identity")),
            AcceptSwapRequestHttpBody::OnlyRedeem { alpha_ledger_redeem_identity } => Ok(StateMachineResponse {
                alpha_ledger_redeem_identity,
                beta_ledger_refund_identity: secret_source.new_secp256k1_refund(id),
            }),
        }
    }
}

#[derive(Deserialize)]
struct DeclineSwapRequestHttpBody {
    reason: Option<SwapDeclineReason>,
}
