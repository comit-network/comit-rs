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
                log::error!("IntoResponsePayload is not available for Accept/Decline");
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
    Accept: ToSirenAction,
    Decline: ToSirenAction,
    Deploy: ListRequiredFields,
    Fund: ListRequiredFields,
    Redeem: ListRequiredFields,
    Refund: ListRequiredFields,
{
    fn to_siren_action(&self, id: &SwapId) -> siren::Action {
        let (name, fields) = match self {
            Action::Deploy(_) => ("deploy", Deploy::list_required_fields()),
            Action::Fund(_) => ("fund", Fund::list_required_fields()),
            Action::Redeem(_) => ("redeem", Redeem::list_required_fields()),
            Action::Refund(_) => ("refund", Refund::list_required_fields()),
            Action::Decline(decline) => return decline.to_siren_action(id),
            Action::Accept(accept) => return accept.to_siren_action(id),
        };

        siren::Action {
            name: name.to_owned(),
            href: new_action_link(id, name),
            method: Some(http::Method::from(ActionKind::from(self))),
            _type: Some("application/x-www-form-urlencoded".to_owned()),
            fields,
            class: vec![],
            title: None,
        }
    }
}
