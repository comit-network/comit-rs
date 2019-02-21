use crate::{
    http_api::{
        rfc003::handlers::get_swap::SwapStatus,
        route_factory::{swap_path, RFC003},
    },
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        metadata_store::RoleKind,
        rfc003::state_store::StateStore,
        Metadata, MetadataStore, SwapId,
    },
};
use ethereum_support::Erc20Token;
use http_api_problem::HttpApiProblem;
use rustic_hal::HalResource;

#[derive(Serialize, Debug)]
pub struct EmbeddedSwapResource {
    state: SwapStatus,
    protocol: String,
}

pub fn handle_get_swaps<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
) -> Result<Vec<HalResource>, HttpApiProblem> {
    let mut resources = vec![];
    for (id, metadata) in metadata_store.all()?.into_iter() {
        with_swap_types!(
            &metadata,
            (|| -> Result<(), HttpApiProblem> {
                let state = state_store.get::<ROLE>(id)?;

                match state {
                    Some(state) => {
                        // TODO: Implement From<actor::State> for SwapOutcome
                        let communication = state.swap_communication.clone().into();
                        let alpha_ledger = state.alpha_ledger_state.clone().into();
                        let beta_ledger = state.beta_ledger_state.clone().into();
                        let error = state.error;
                        let state =
                            SwapStatus::new(&communication, &alpha_ledger, &beta_ledger, &error);

                        let swap = EmbeddedSwapResource {
                            state,
                            protocol: RFC003.into(),
                        };

                        let mut hal_resource = HalResource::new(swap);
                        hal_resource.with_link("self", swap_path(id));
                        resources.push(hal_resource);
                    }
                    None => error!("Couldn't find state for {} despite having the metadata", id),
                };
                Ok(())
            })
        )?;
    }

    Ok(resources)
}
