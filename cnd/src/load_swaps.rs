#![allow(clippy::type_repetition_in_bounds)]
use crate::{
    db::{DetermineTypes, LoadAcceptedSwap, Retrieve},
    init_swap::init_accepted_swap,
    swap_protocols::Facade,
};

#[allow(clippy::cognitive_complexity)]
pub async fn load_swaps_from_database(facade: Facade) -> anyhow::Result<()> {
    tracing::debug!("loading swaps from database ...");

    for swap in Retrieve::all(&facade).await?.iter() {
        let swap_id = swap.swap_id;
        tracing::debug!("got swap from database: {}", swap_id);

        let types = DetermineTypes::determine_types(&facade, &swap_id).await?;

        with_swap_types!(types, {
            let accepted =
                LoadAcceptedSwap::<AL, BL, AA, BA>::load_accepted_swap(&facade, &swap_id).await;

            match accepted {
                Ok(accepted) => {
                    init_accepted_swap(&facade, accepted, types.role)?;
                }
                Err(e) => tracing::error!("failed to load swap: {}, continuing ...", e),
            };
        });
    }
    Ok(())
}
