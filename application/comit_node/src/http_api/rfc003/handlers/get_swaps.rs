use crate::{
    http_api::{
        rfc003::{
            action::{new_action_link, Action, ToAction},
            handlers::get_swap::{SwapParameters, SwapStatus},
        },
        route_factory::{swap_path, RFC003},
        Http,
    },
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        metadata_store::RoleKind,
        rfc003::{state_store::StateStore, Actions, Ledger},
        Metadata, MetadataStore, SwapId,
    },
};
use ethereum_support::Erc20Token;
use http_api_problem::HttpApiProblem;
use rustic_hal::HalResource;
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(
    bound = "Http<AL>: Serialize, Http<BL>: Serialize, Http<AA>: Serialize, Http<BA>: Serialize,\
             Http<AL::Identity>: Serialize, Http<BL::Identity>: Serialize,\
             Http<AL::HtlcLocation>: Serialize, Http<BL::HtlcLocation>: Serialize,\
             Http<AL::Transaction>: Serialize, Http<BL::Transaction>: Serialize"
)]
pub struct EmbeddedSwapResource<AL: Ledger, BL: Ledger, AA, BA> {
    status: SwapStatus,
    protocol: String,
    parameters: SwapParameters<AL, BL, AA, BA>,
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
                        let parameters = SwapParameters::new(state.clone().request());
                        let actions: Vec<Action> = state
                            .actions()
                            .iter()
                            .map(|action| action.to_action())
                            .collect();
                        let error = state.error;
                        let status = SwapStatus::new::<AL, BL>(
                            &communication,
                            &alpha_ledger,
                            &beta_ledger,
                            &error,
                        );

                        let swap = EmbeddedSwapResource {
                            status,
                            protocol: RFC003.into(),
                            parameters,
                        };

                        let mut hal_resource = HalResource::new(swap);
                        hal_resource.with_link("self", swap_path(id));

                        for action in actions {
                            let link = new_action_link(&id, action);
                            hal_resource.with_link(action, link);
                        }

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
