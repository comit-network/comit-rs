use crate::{
    comit_client::SwapDeclineReason,
    http_api::problem,
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            messages::{AcceptResponseBody, ToAcceptResponseBody},
            state_store::StateStore,
            Actions, Ledger, SecretSource,
        },
        MetadataStore, SwapId,
    },
};
use bitcoin_support;
use ethereum_support::{self, Erc20Token};
use http_api_problem::HttpApiProblem;
use std::str::FromStr;

#[allow(clippy::unit_arg, clippy::let_unit_value)]
pub fn handle_post_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
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
            PostAction::Accept => serde_json::from_value::<BobAcceptBody>(body)
                .map_err(|e| {
                    error!(
                        "Failed to deserialize body of accept response for swap {}: {:?}",
                        id, e
                    );
                    problem::serde(&e)
                })
                .and_then(|accept_body| {
                    let state = state_store
                        .get::<ROLE>(id)?
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

                    accept_action
                        .accept(accept_body)
                        .map_err(|_| problem::action_already_taken())
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
                            .get::<ROLE>(id)?
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

                        decline_action
                            .decline(reason)
                            .map_err(|_| problem::action_already_taken())
                    })
            }
        })
    )
}

#[derive(Clone, Copy, Debug)]
pub enum PostAction {
    Accept,
    Decline,
}

#[derive(Deserialize, Clone, Debug)]
struct OnlyRedeem<L: Ledger> {
    pub alpha_ledger_redeem_identity: L::Identity,
}

#[derive(Deserialize, Clone, Debug)]
struct OnlyRefund<L: Ledger> {
    pub beta_ledger_refund_identity: L::Identity,
}

#[derive(Deserialize)]
struct DeclineSwapRequestHttpBody {
    reason: Option<SwapDeclineReason>,
}

impl ToAcceptResponseBody<Bitcoin, Ethereum> for OnlyRefund<Ethereum> {
    fn to_accept_response_body(
        &self,
        secret_source: &dyn SecretSource,
    ) -> AcceptResponseBody<Bitcoin, Ethereum> {
        AcceptResponseBody {
            beta_ledger_refund_identity: self.beta_ledger_refund_identity,
            alpha_ledger_redeem_identity: secret_source.secp256k1_redeem().into(),
        }
    }
}

impl ToAcceptResponseBody<Ethereum, Bitcoin> for OnlyRedeem<Ethereum> {
    fn to_accept_response_body(
        &self,
        secret_source: &dyn SecretSource,
    ) -> AcceptResponseBody<Ethereum, Bitcoin> {
        AcceptResponseBody {
            alpha_ledger_redeem_identity: self.alpha_ledger_redeem_identity,
            beta_ledger_refund_identity: secret_source.secp256k1_refund().into(),
        }
    }
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
