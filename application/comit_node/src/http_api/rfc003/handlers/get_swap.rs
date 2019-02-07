use crate::{
    http_api::{
        asset::{HttpAsset, ToHttpAsset},
        ledger::{HttpLedger, ToHttpLedger},
        problem,
    },
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        metadata_store::RoleKind,
        rfc003::{state_store::StateStore, Actions, Alice, Bob, Timestamp},
        Metadata, MetadataStore, SwapId,
    },
};
use ethereum_support::Erc20Token;
use http_api_problem::HttpApiProblem;
use std::sync::Arc;

pub fn handle_get_swap<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: &Arc<T>,
    state_store: &Arc<S>,
    id: &SwapId,
) -> Result<(GetSwapResource, Vec<ActionName>), HttpApiProblem> {
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

            let start_state = state.start_state()?;

            let actions: Vec<ActionName> =
                state.actions().iter().map(|action| action.name()).collect();
            (Ok((
                GetSwapResource {
                    state: state.name(),
                    swap: SwapDescription {
                        alpha_ledger: start_state.alpha_ledger.to_http_ledger().unwrap(),
                        beta_ledger: start_state.beta_ledger.to_http_ledger().unwrap(),
                        alpha_asset: start_state.alpha_asset.to_http_asset().unwrap(),
                        beta_asset: start_state.beta_asset.to_http_asset().unwrap(),
                        alpha_expiry: start_state.alpha_expiry,
                        beta_expiry: start_state.beta_expiry,
                    },
                    role: format!("{}", metadata.role),
                },
                actions,
            )))
        })
    )
}

type ActionName = String;

#[derive(Debug, Serialize)]
pub struct SwapDescription {
    alpha_ledger: HttpLedger,
    beta_ledger: HttpLedger,
    alpha_asset: HttpAsset,
    beta_asset: HttpAsset,
    alpha_expiry: Timestamp,
    beta_expiry: Timestamp,
}

#[derive(Debug, Serialize)]
pub struct GetSwapResource {
    swap: SwapDescription,
    role: String,
    state: String,
}
