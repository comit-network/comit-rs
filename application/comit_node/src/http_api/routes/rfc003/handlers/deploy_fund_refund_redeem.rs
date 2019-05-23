use crate::{
    http_api::{
        action::{
            ActionExecutionParameters, ActionResponseBody, IntoResponsePayload, ListRequiredFields,
            ToSirenAction,
        },
        problem,
        routes::rfc003::action::new_action_link,
    },
    swap_protocols::{
        actions::Actions,
        metadata_store::Metadata,
        rfc003::{actions::ActionKind, state_store::StateStore},
        MetadataStore, SwapId,
    },
};
use bitcoin_support;
use ethereum_support;
use http_api_problem::{HttpApiProblem, StatusCode};

pub fn handle_deploy_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
    id: &SwapId,
    query_params: &ActionExecutionParameters,
) -> Result<ActionResponseBody, HttpApiProblem> {
    let metadata = metadata_store
        .get(id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<ROLE>(id.clone())?
                .ok_or_else(problem::state_store)?;
            log::trace!("Retrieved state for {}: {:?}", id, state);

            state
                .actions()
                .into_iter()
                .find_map(|action| match action {
                    ActionKind::Deploy(action) => {
                        Some(action.into_response_payload(query_params.clone()))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| {
                    //                Err(problem::invalid_action(action_name))
                    unimplemented!()
                })
        })
    )
}
pub fn handle_fund_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
    id: &SwapId,
    query_params: &ActionExecutionParameters,
) -> Result<ActionResponseBody, HttpApiProblem> {
    let metadata = metadata_store
        .get(id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<ROLE>(id.clone())?
                .ok_or_else(problem::state_store)?;
            log::trace!("Retrieved state for {}: {:?}", id, state);

            state
                .actions()
                .into_iter()
                .find_map(|action| match action {
                    ActionKind::Fund(action) => {
                        Some(action.into_response_payload(query_params.clone()))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| {
                    //                Err(problem::invalid_action(action_name))
                    unimplemented!()
                })
        })
    )
}
pub fn handle_refund_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
    id: &SwapId,
    query_params: &ActionExecutionParameters,
) -> Result<ActionResponseBody, HttpApiProblem> {
    let metadata = metadata_store
        .get(id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<ROLE>(id.clone())?
                .ok_or_else(problem::state_store)?;
            log::trace!("Retrieved state for {}: {:?}", id, state);

            state
                .actions()
                .into_iter()
                .find_map(|action| match action {
                    ActionKind::Refund(action) => {
                        Some(action.into_response_payload(query_params.clone()))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| {
                    //                Err(problem::invalid_action(action_name))
                    unimplemented!()
                })
        })
    )
}
pub fn handle_redeem_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
    id: &SwapId,
    query_params: &ActionExecutionParameters,
) -> Result<ActionResponseBody, HttpApiProblem> {
    let metadata = metadata_store
        .get(id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<ROLE>(id.clone())?
                .ok_or_else(problem::state_store)?;
            log::trace!("Retrieved state for {}: {:?}", id, state);

            state
                .actions()
                .into_iter()
                .find_map(|action| match action {
                    ActionKind::Redeem(action) => {
                        Some(action.into_response_payload(query_params.clone()))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| {
                    //                Err(problem::invalid_action(action_name))
                    unimplemented!()
                })
        })
    )
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> IntoResponsePayload
    for ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>
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
            ActionKind::Deploy(payload) => payload.into_response_payload(query_params),
            ActionKind::Fund(payload) => payload.into_response_payload(query_params),
            ActionKind::Redeem(payload) => payload.into_response_payload(query_params),
            ActionKind::Refund(payload) => payload.into_response_payload(query_params),
            ActionKind::Accept(_) | ActionKind::Decline(_) => {
                log::error!("IntoResponsePayload is not available for Accept/Decline");
                Err(HttpApiProblem::with_title_and_type_from_status(
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> ToSirenAction
    for ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>
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
            ActionKind::Deploy(_) => ("deploy", Deploy::list_required_fields()),
            ActionKind::Fund(_) => ("fund", Fund::list_required_fields()),
            ActionKind::Redeem(_) => ("redeem", Redeem::list_required_fields()),
            ActionKind::Refund(_) => ("refund", Refund::list_required_fields()),
            ActionKind::Decline(decline) => return decline.to_siren_action(id),
            ActionKind::Accept(accept) => return accept.to_siren_action(id),
        };

        siren::Action {
            name: name.to_owned(),
            href: new_action_link(id, name),
            method: Some(http::Method::GET),
            _type: Some("application/x-www-form-urlencoded".to_owned()),
            fields,
            class: vec![],
            title: None,
        }
    }
}
