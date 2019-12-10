use crate::{
    db::{DetermineTypes, Save, Saver},
    ethereum::{Erc20Token, EtherQuantity},
    http_api::{
        action::{
            ActionExecutionParameters, ActionResponseBody, IntoResponsePayload, ListRequiredFields,
            ToSirenAction,
        },
        route_factory::new_action_link,
        routes::rfc003::decline::{to_swap_decline_reason, DeclineBody},
    },
    libp2p_comit_ext::ToHeader,
    network::Network,
    seed::SwapSeed,
    swap_protocols::{
        self,
        actions::Actions,
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            self,
            actions::{Action, ActionKind},
            bob::State,
            events::HtlcEvents,
            messages::{Decision, IntoAcceptMessage},
            state_store::StateStore,
        },
        SwapId,
    },
};
use anyhow::Context;
use bitcoin::Amount;
use libp2p_comit::frame::Response;
use std::fmt::Debug;
use tokio::executor::Executor;
use warp::http;

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
pub async fn handle_action<
    D: StateStore
        + Network
        + SwapSeed
        + Saver
        + DetermineTypes
        + HtlcEvents<Bitcoin, Amount>
        + HtlcEvents<Ethereum, EtherQuantity>
        + HtlcEvents<Ethereum, Erc20Token>
        + Executor
        + Clone,
>(
    method: http::Method,
    swap_id: SwapId,
    action_kind: ActionKind,
    body: serde_json::Value,
    query_params: ActionExecutionParameters,
    dependencies: D,
) -> anyhow::Result<ActionResponseBody> {
    let types = dependencies.determine_types(&swap_id).await?;

    with_swap_types!(types, {
        let state = StateStore::get::<ROLE>(&dependencies, &swap_id)?.ok_or_else(|| {
            anyhow::anyhow!("state store did not contain an entry for {}", swap_id)
        })?;
        log::trace!("Retrieved state for {}: {:?}", swap_id, state);

        let action = state
            .actions()
            .into_iter()
            .select_action(action_kind, method)?;

        match action {
            Action::Accept(_) => {
                let body = serde_json::from_value::<AcceptBody>(body)
                    .context("failed to deserialize accept body")?;

                let channel =
                    Network::pending_request_for(&dependencies, swap_id).with_context(|| {
                        format!("unable to find response channel for swap {}", swap_id)
                    })?;

                let accept_message =
                    body.into_accept_message(swap_id, &SwapSeed::swap_seed(&dependencies, swap_id));

                Save::save(&dependencies, accept_message).await?;

                let response = rfc003_accept_response(accept_message);
                channel.send(response).map_err(|_| {
                    anyhow::anyhow!(
                        "failed to send response through channel for swap {}",
                        swap_id
                    )
                })?;

                let swap_request = state.request();
                swap_protocols::init_accepted_swap(
                    &dependencies,
                    swap_request,
                    accept_message,
                    types.role,
                )?;

                Ok(ActionResponseBody::None)
            }
            Action::Decline(_) => {
                let body = serde_json::from_value::<DeclineBody>(body)?;

                let channel =
                    Network::pending_request_for(&dependencies, swap_id).with_context(|| {
                        format!("unable to find response channel for swap {}", swap_id)
                    })?;

                let decline_message = rfc003::Decline {
                    swap_id,
                    reason: to_swap_decline_reason(body.reason),
                };

                Save::save(&dependencies, decline_message.clone()).await?;

                let response = rfc003_decline_response(decline_message.clone());
                channel.send(response).map_err(|_| {
                    anyhow::anyhow!(
                        "failed to send response through channel for swap {}",
                        swap_id
                    )
                })?;

                let swap_request = state.request();
                let seed = dependencies.swap_seed(swap_id);
                let state = State::declined(swap_request.clone(), decline_message.clone(), seed);
                StateStore::insert(&dependencies, swap_id, state);

                Ok(ActionResponseBody::None)
            }
            Action::Deploy(action) => action.into_response_payload(query_params),
            Action::Fund(action) => action.into_response_payload(query_params),
            Action::Redeem(action) => action.into_response_payload(query_params),
            Action::Refund(action) => action.into_response_payload(query_params),
        }
    })
}

#[derive(Debug, thiserror::Error, PartialEq)]
#[error("attempt to invoke {action_kind} action with http method {method}, which is an invalid combination")]
pub struct InvalidActionInvocation {
    action_kind: ActionKind,
    method: http::Method,
}

#[derive(Debug, thiserror::Error, PartialEq)]
#[error("action {action_kind} is invalid for this swap")]
pub struct InvalidAction {
    action_kind: ActionKind,
}

trait SelectAction<Accept, Decline, Deploy, Fund, Redeem, Refund>:
    Iterator<Item = Action<Accept, Decline, Deploy, Fund, Redeem, Refund>>
{
    fn select_action(
        mut self,
        action_kind: ActionKind,
        method: http::Method,
    ) -> anyhow::Result<Self::Item>
    where
        Self: Sized,
    {
        let action = self
            .find(|action| ActionKind::from(action) == action_kind)
            .ok_or_else(|| anyhow::Error::from(InvalidAction { action_kind }))?;

        if http::Method::from(action_kind) != method {
            return Err(anyhow::Error::from(InvalidActionInvocation {
                action_kind,
                method,
            }));
        }

        Ok(action)
    }
}

fn rfc003_accept_response<AL: rfc003::Ledger, BL: rfc003::Ledger>(
    message: rfc003::messages::Accept<AL, BL>,
) -> Response {
    Response::empty()
        .with_header(
            "decision",
            Decision::Accepted
                .to_header()
                .expect("Decision should not fail to serialize"),
        )
        .with_body(
            serde_json::to_value(rfc003::messages::AcceptResponseBody::<AL, BL> {
                beta_ledger_refund_identity: message.beta_ledger_refund_identity,
                alpha_ledger_redeem_identity: message.alpha_ledger_redeem_identity,
            })
            .expect("body should always serialize into serde_json::Value"),
        )
}

fn rfc003_decline_response(message: rfc003::messages::Decline) -> Response {
    Response::empty()
        .with_header(
            "decision",
            Decision::Declined
                .to_header()
                .expect("Decision shouldn't fail to serialize"),
        )
        .with_body(
            serde_json::to_value(rfc003::messages::DeclineResponseBody {
                reason: message.reason,
            })
            .expect("decline body should always serialize into serde_json::Value"),
        )
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund, I>
    SelectAction<Accept, Decline, Deploy, Fund, Redeem, Refund> for I
where
    I: Iterator<Item = Action<Accept, Decline, Deploy, Fund, Redeem, Refund>>,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral_ext::AnyhowResultAssertions;
    use spectral::prelude::*;

    fn actions() -> Vec<Action<(), (), (), (), (), ()>> {
        Vec::new()
    }

    #[test]
    fn action_not_available_should_return_409_conflict() {
        let given_actions = actions();

        let result = given_actions
            .into_iter()
            .select_action(ActionKind::Accept, http::Method::POST);

        assert_that(&result)
            .is_inner_err::<InvalidAction>()
            .is_equal_to(&InvalidAction {
                action_kind: ActionKind::Accept,
            });
    }

    #[test]
    fn accept_decline_action_should_be_returned_with_http_post() {
        let mut given_actions = actions();
        given_actions.extend(vec![Action::Accept(()), Action::Decline(())]);

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Accept, http::Method::POST);

        assert_that(&result).is_ok_containing(Action::Accept(()));

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Decline, http::Method::POST);

        assert_that(&result).is_ok_containing(Action::Decline(()));
    }

    #[test]
    fn accept_decline_action_cannot_be_invoked_with_http_get() {
        let mut given_actions = actions();
        given_actions.extend(vec![Action::Accept(()), Action::Decline(())]);

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Accept, http::Method::GET);

        assert_that(&result)
            .is_inner_err::<InvalidActionInvocation>()
            .is_equal_to(&InvalidActionInvocation {
                action_kind: ActionKind::Accept,
                method: http::Method::GET,
            });

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Decline, http::Method::GET);

        assert_that(&result)
            .is_inner_err::<InvalidActionInvocation>()
            .is_equal_to(&InvalidActionInvocation {
                action_kind: ActionKind::Decline,
                method: http::Method::GET,
            });
    }

    #[test]
    fn deploy_fund_refund_redeem_action_cannot_be_invoked_with_http_post() {
        let mut given_actions = actions();
        given_actions.extend(vec![
            Action::Deploy(()),
            Action::Fund(()),
            Action::Refund(()),
            Action::Redeem(()),
        ]);

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Deploy, http::Method::POST);

        assert_that(&result)
            .is_inner_err::<InvalidActionInvocation>()
            .is_equal_to(&InvalidActionInvocation {
                action_kind: ActionKind::Deploy,
                method: http::Method::POST,
            });

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Fund, http::Method::POST);

        assert_that(&result)
            .is_inner_err::<InvalidActionInvocation>()
            .is_equal_to(&InvalidActionInvocation {
                action_kind: ActionKind::Fund,
                method: http::Method::POST,
            });

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Refund, http::Method::POST);

        assert_that(&result)
            .is_inner_err::<InvalidActionInvocation>()
            .is_equal_to(&InvalidActionInvocation {
                action_kind: ActionKind::Refund,
                method: http::Method::POST,
            });

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Redeem, http::Method::POST);

        assert_that(&result)
            .is_inner_err::<InvalidActionInvocation>()
            .is_equal_to(&InvalidActionInvocation {
                action_kind: ActionKind::Redeem,
                method: http::Method::POST,
            });
    }
}

impl From<ActionKind> for http::Method {
    fn from(action_kind: ActionKind) -> Self {
        match action_kind {
            ActionKind::Accept => http::Method::POST,
            ActionKind::Decline => http::Method::POST,
            ActionKind::Deploy => http::Method::GET,
            ActionKind::Fund => http::Method::GET,
            ActionKind::Refund => http::Method::GET,
            ActionKind::Redeem => http::Method::GET,
        }
    }
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> IntoResponsePayload
    for Action<Accept, Decline, Deploy, Fund, Redeem, Refund>
where
    Deploy: IntoResponsePayload,
    Fund: IntoResponsePayload,
    Redeem: IntoResponsePayload,
    Refund: IntoResponsePayload,
{
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> anyhow::Result<ActionResponseBody> {
        match self {
            Action::Deploy(payload) => payload.into_response_payload(query_params),
            Action::Fund(payload) => payload.into_response_payload(query_params),
            Action::Redeem(payload) => payload.into_response_payload(query_params),
            Action::Refund(payload) => payload.into_response_payload(query_params),
            Action::Accept(_) | Action::Decline(_) => Err(anyhow::anyhow!(
                "IntoResponsePayload is not available for Accept/Decline"
            )),
        }
    }
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> ToSirenAction
    for Action<Accept, Decline, Deploy, Fund, Redeem, Refund>
where
    Accept: ListRequiredFields + Debug,
    Decline: ListRequiredFields + Debug,
    Deploy: ListRequiredFields + Debug,
    Fund: ListRequiredFields + Debug,
    Redeem: ListRequiredFields + Debug,
    Refund: ListRequiredFields + Debug,
{
    fn to_siren_action(&self, id: &SwapId) -> siren::Action {
        let action_kind = ActionKind::from(self);
        let method = http::Method::from(action_kind);
        let name = action_kind.to_string();

        let media_type = match method {
            // GET + DELETE cannot have a body
            http::Method::GET | http::Method::DELETE => None,
            _ => Some("application/json".to_owned()),
        };

        let fields = match self {
            Action::Accept(_) => Accept::list_required_fields(),
            Action::Decline(_) => Decline::list_required_fields(),
            Action::Deploy(_) => Deploy::list_required_fields(),
            Action::Fund(_) => Fund::list_required_fields(),
            Action::Redeem(_) => Redeem::list_required_fields(),
            Action::Refund(_) => Refund::list_required_fields(),
        };

        log::debug!(target: "http-api", "Creating siren::Action from {:?} with HTTP method: {}, Media-Type: {:?}, Name: {}, Fields: {:?}", self, method, media_type, name, fields);

        siren::Action {
            href: new_action_link(id, &name),
            name,
            method: Some(method),
            _type: media_type,
            fields,
            class: vec![],
            title: None,
        }
    }
}
