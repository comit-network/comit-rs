use crate::{
    http_api::{
        action::{
            ActionExecutionParameters, ActionResponseBody, IntoResponsePayload, ListRequiredFields,
            ToSirenAction,
        },
        problem,
        route_factory::new_action_link,
        routes::rfc003::decline::{to_swap_decline_reason, DeclineBody},
    },
    libp2p_comit_ext::ToHeader,
    network::Network,
    swap_protocols::{
        actions::Actions,
        rfc003::{
            self,
            actions::{Action, ActionKind},
            bob::BobSpawner,
            messages::Decision,
            state_store::StateStore,
        },
        MetadataStore, SwapId,
    },
};
use http_api_problem::HttpApiProblem;
use libp2p_comit::frame::Response;
use std::fmt::Debug;

#[allow(clippy::unit_arg, clippy::let_unit_value)]
pub fn handle_action<D: MetadataStore + StateStore + BobSpawner + Network>(
    method: http::Method,
    id: SwapId,
    action_kind: ActionKind,
    body: serde_json::Value,
    query_params: ActionExecutionParameters,
    dependencies: D,
) -> Result<ActionResponseBody, HttpApiProblem> {
    let metadata = MetadataStore::get(&dependencies, id)?.ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state =
                StateStore::get::<ROLE>(&dependencies, &id)?.ok_or_else(problem::state_store)?;
            log::trace!("Retrieved state for {}: {:?}", id, state);

            state
                .actions()
                .into_iter()
                .select_action(action_kind, method)
                .and_then({
                    |action| match action {
                        Action::Accept(action) => serde_json::from_value::<AcceptBody>(body)
                            .map_err(problem::deserialize)
                            .and_then({
                                |body| {
                                    let channel = Network::pending_request_for(&dependencies, id)
                                        .ok_or_else(problem::missing_channel)?;

                                    let accept_body = action.accept(body);

                                    let response = rfc003_accept_response(accept_body.clone());
                                    channel.send(response).map_err(problem::send_over_channel)?;

                                    let request = state.request();
                                    BobSpawner::spawn(&dependencies, request, Ok(accept_body));

                                    Ok(ActionResponseBody::None)
                                }
                            }),
                        Action::Decline(action) => serde_json::from_value::<DeclineBody>(body)
                            .map_err(problem::deserialize)
                            .and_then({
                                |body| {
                                    let channel = Network::pending_request_for(&dependencies, id)
                                        .ok_or_else(problem::missing_channel)?;

                                    let decline_body =
                                        action.decline(to_swap_decline_reason(body.reason));

                                    let response = rfc003_decline_response(decline_body.clone());
                                    channel.send(response).map_err(problem::send_over_channel)?;

                                    let request = state.request();
                                    BobSpawner::spawn(&dependencies, request, Err(decline_body));

                                    Ok(ActionResponseBody::None)
                                }
                            }),
                        Action::Deploy(action) => action.into_response_payload(query_params),
                        Action::Fund(action) => action.into_response_payload(query_params),
                        Action::Redeem(action) => action.into_response_payload(query_params),
                        Action::Refund(action) => action.into_response_payload(query_params),
                    }
                })
        })
    )
}

trait SelectAction<Accept, Decline, Deploy, Fund, Redeem, Refund>:
    Iterator<Item = Action<Accept, Decline, Deploy, Fund, Redeem, Refund>>
{
    fn select_action(
        mut self,
        action_kind: ActionKind,
        method: http::Method,
    ) -> Result<Self::Item, HttpApiProblem>
    where
        Self: Sized,
    {
        self.find(|action| ActionKind::from(action) == action_kind)
            .ok_or_else(|| problem::invalid_action(action_kind))
            .and_then(|action| {
                if http::Method::from(action_kind) != method {
                    log::debug!(target: "http-api", "Attempt to invoke {} action with http method {}, which is an invalid combination.", action_kind, method);
                    return Err(HttpApiProblem::new("Invalid action invocation")
                        .set_status(http::StatusCode::METHOD_NOT_ALLOWED));
                }

                Ok(action)
            })
    }
}

fn rfc003_accept_response<AL: rfc003::Ledger, BL: rfc003::Ledger>(
    body: rfc003::messages::AcceptResponseBody<AL, BL>,
) -> Response {
    Response::empty()
        .with_header(
            "decision",
            Decision::Accepted
                .to_header()
                .expect("Decision should not fail to serialize"),
        )
        .with_body(
            serde_json::to_value(body)
                .expect("body should always serialize into serde_json::Value"),
        )
}

fn rfc003_decline_response(body: rfc003::messages::DeclineResponseBody) -> Response {
    Response::empty()
        .with_header(
            "decision",
            Decision::Declined
                .to_header()
                .expect("Decision shouldn't fail to serialize"),
        )
        .with_body(
            serde_json::to_value(body)
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
            .is_err()
            .map(|p| &p.status)
            .is_equal_to(Some(http::StatusCode::CONFLICT));
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
            .is_err()
            .map(|p| &p.status)
            .is_equal_to(Some(http::StatusCode::METHOD_NOT_ALLOWED));

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Decline, http::Method::GET);

        assert_that(&result)
            .is_err()
            .map(|p| &p.status)
            .is_equal_to(Some(http::StatusCode::METHOD_NOT_ALLOWED));
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
            .is_err()
            .map(|p| &p.status)
            .is_equal_to(Some(http::StatusCode::METHOD_NOT_ALLOWED));

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Fund, http::Method::POST);

        assert_that(&result)
            .is_err()
            .map(|p| &p.status)
            .is_equal_to(Some(http::StatusCode::METHOD_NOT_ALLOWED));

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Refund, http::Method::POST);

        assert_that(&result)
            .is_err()
            .map(|p| &p.status)
            .is_equal_to(Some(http::StatusCode::METHOD_NOT_ALLOWED));

        let result = given_actions
            .clone()
            .into_iter()
            .select_action(ActionKind::Redeem, http::Method::POST);

        assert_that(&result)
            .is_err()
            .map(|p| &p.status)
            .is_equal_to(Some(http::StatusCode::METHOD_NOT_ALLOWED));
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
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match self {
            Action::Deploy(payload) => payload.into_response_payload(query_params),
            Action::Fund(payload) => payload.into_response_payload(query_params),
            Action::Redeem(payload) => payload.into_response_payload(query_params),
            Action::Refund(payload) => payload.into_response_payload(query_params),
            Action::Accept(_) | Action::Decline(_) => {
                log::error!(target: "http-api", "IntoResponsePayload is not available for Accept/Decline");
                Err(HttpApiProblem::with_title_and_type_from_status(
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
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
