use crate::{
    http_api::{
        action::{
            ActionExecutionParameters, ActionResponseBody, IntoResponsePayload, ListRequiredFields,
            ToSirenAction,
        },
        problem,
        route_factory::new_action_link,
        routes::rfc003::decline::DeclineBody,
    },
    swap_protocols::{
        actions::Actions,
        rfc003::{
            actions::{Action, ActionKind},
            state_store::StateStore,
        },
        MetadataStore, SwapId,
    },
};
use http_api_problem::HttpApiProblem;
use std::fmt::Debug;

#[allow(clippy::unit_arg, clippy::let_unit_value)]
pub fn handle_action<T: MetadataStore<SwapId>, S: StateStore>(
    method: http::Method,
    id: SwapId,
    action_kind: ActionKind,
    body: serde_json::Value,
    query_params: ActionExecutionParameters,
    metadata_store: &T,
    state_store: &S,
) -> Result<ActionResponseBody, HttpApiProblem> {
    let metadata = metadata_store
        .get(&id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<ROLE>(&id)?
                .ok_or_else(problem::state_store)?;
            log::trace!("Retrieved state for {}: {:?}", id, state);

            if http::Method::from(action_kind) != method {
                log::debug!(target: "http-api", "Attempt to invoke {} action with http method {}, which is an invalid combination.", action_kind, method);
                return Err(HttpApiProblem::new("Invalid action invocation")
                    .set_status(http::StatusCode::METHOD_NOT_ALLOWED));
            }

            state
                .actions()
                .into_iter()
                .find(|action| ActionKind::from(action) == action_kind)
                .map(|action| match action {
                    Action::Accept(action) => serde_json::from_value::<AcceptBody>(body)
                        .map_err(problem::deserialize)
                        .and_then(|body| {
                            action
                                .accept(body)
                                .map(|_| ActionResponseBody::None)
                                .map_err(|_| problem::action_already_done(action_kind))
                        }),

                    Action::Decline(action) => serde_json::from_value::<DeclineBody>(body)
                        .map_err(problem::deserialize)
                        .and_then(|body| {
                            action
                                .decline(body.reason)
                                .map(|_| ActionResponseBody::None)
                                .map_err(|_| problem::action_already_done(action_kind))
                        }),
                    Action::Deploy(action) => action.into_response_payload(query_params),
                    Action::Fund(action) => action.into_response_payload(query_params),
                    Action::Redeem(action) => action.into_response_payload(query_params),
                    Action::Refund(action) => action.into_response_payload(query_params),
                })
                .unwrap_or_else(|| Err(problem::invalid_action(action_kind)))
        })
    )
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
            http::Method::GET => "application/x-www-form-urlencoded",
            http::Method::DELETE => "application/x-www-form-urlencoded",
            _ => "application/json",
        };

        let fields = match self {
            Action::Accept(_) => Accept::list_required_fields(),
            Action::Decline(_) => Decline::list_required_fields(),
            Action::Deploy(_) => Deploy::list_required_fields(),
            Action::Fund(_) => Fund::list_required_fields(),
            Action::Redeem(_) => Redeem::list_required_fields(),
            Action::Refund(_) => Refund::list_required_fields(),
        };

        log::debug!(target: "http-api", "Creating siren::Action from {:?} with HTTP method: {}, Media-Type: {}, Name: {}, Fields: {:?}", self, method, media_type, name, fields);

        siren::Action {
            href: new_action_link(id, &name),
            name,
            method: Some(method),
            _type: Some(media_type.to_owned()),
            fields,
            class: vec![],
            title: None,
        }
    }
}
