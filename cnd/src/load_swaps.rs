#![allow(clippy::type_repetition_in_bounds)]
use crate::{
    asset,
    db::{DetermineTypes, LoadAcceptedSwap, Retrieve},
    init_swap::init_accepted_swap,
    seed::DeriveSwapSeed,
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{events::HtlcEvents, state_store::StateStore},
    },
};
use bitcoin::Amount;

#[allow(clippy::cognitive_complexity)]
pub async fn load_swaps_from_database<D>(dependencies: D) -> anyhow::Result<()>
where
    D: StateStore
        + Clone
        + DeriveSwapSeed
        + Retrieve
        + DetermineTypes
        + HtlcEvents<Bitcoin, Amount>
        + HtlcEvents<Ethereum, asset::Ether>
        + HtlcEvents<Ethereum, asset::Erc20>
        + LoadAcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, asset::Ether>
        + LoadAcceptedSwap<Ethereum, Bitcoin, asset::Ether, bitcoin::Amount>
        + LoadAcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, asset::Erc20>
        + LoadAcceptedSwap<Ethereum, Bitcoin, asset::Erc20, bitcoin::Amount>,
{
    log::debug!("loading swaps from database ...");

    for swap in Retrieve::all(&dependencies).await?.iter() {
        let swap_id = swap.swap_id;
        log::debug!("got swap from database: {}", swap_id);

        let types = DetermineTypes::determine_types(&dependencies, &swap_id).await?;

        with_swap_types!(types, {
            let accepted =
                LoadAcceptedSwap::<AL, BL, AA, BA>::load_accepted_swap(&dependencies, &swap_id)
                    .await;

            match accepted {
                Ok((request, accept, _at)) => {
                    init_accepted_swap(&dependencies, request, accept, types.role)?;
                }
                Err(e) => log::error!("failed to load swap: {}, continuing ...", e),
            };
        });
    }
    Ok(())
}
