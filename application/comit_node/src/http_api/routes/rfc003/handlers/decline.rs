use crate::{
    comit_client::SwapDeclineReason,
    http_api::{problem, routes::rfc003::action::ActionName},
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{state_store::StateStore, Actions},
        MetadataStore, SwapId,
    },
};
use bitcoin_support;
use ethereum_support::{self, Erc20Token};
use http_api_problem::{HttpApiProblem, StatusCode as HttpStatusCode};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DeclineSwapRequestHttpBody {
    pub reason: Option<SwapDeclineReason>,
}

#[allow(clippy::unit_arg, clippy::let_unit_value)]
pub fn handle_decline_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
    id: SwapId,
    body: serde_json::Value,
) -> Result<(), HttpApiProblem> {
    use crate::swap_protocols::{Metadata, RoleKind};
    let metadata = metadata_store
        .get(&id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types_bob!(
        &metadata,
        (|| serde_json::from_value::<DeclineSwapRequestHttpBody>(body.clone())
            .map_err(|e| {
                log::error!(
                    "Failed to deserialize body of decline response for swap {}: {:?}",
                    id,
                    e
                );
                problem::deserialize(&e)
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
                        .unwrap_or_else(|| Err(problem::invalid_action(ActionName::Decline)))?
                };

                let reason = decline_body.reason;

                decline_action
                    .decline(reason)
                    .map_err(|_| problem::action_already_done(ActionName::Decline))
            }))
    )
}
