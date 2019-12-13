#![allow(clippy::type_repetition_in_bounds)]
use crate::{
    db::{DetermineTypes, LoadAcceptedSwap, Retrieve},
    ethereum::{Erc20Token, EtherQuantity},
    seed::SwapSeed,
    swap_protocols::{
        self,
        ledger::{Bitcoin, Ethereum},
        rfc003::{events::HtlcEvents, state_store::StateStore},
    },
};
use bitcoin::Amount;
use tokio::executor::Executor;

#[allow(clippy::cognitive_complexity)]
pub async fn load_swaps_from_database<D>(dependencies: D) -> anyhow::Result<()>
where
    D: StateStore
        + Executor
        + Clone
        + SwapSeed
        + Retrieve
        + DetermineTypes
        + HtlcEvents<Bitcoin, Amount>
        + HtlcEvents<Ethereum, EtherQuantity>
        + HtlcEvents<Ethereum, Erc20Token>
        + LoadAcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, EtherQuantity>
        + LoadAcceptedSwap<Ethereum, Bitcoin, EtherQuantity, bitcoin::Amount>
        + LoadAcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, Erc20Token>
        + LoadAcceptedSwap<Ethereum, Bitcoin, Erc20Token, bitcoin::Amount>,
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
                    swap_protocols::init_accepted_swap(&dependencies, request, accept, types.role)?;
                }
                Err(e) => log::error!("failed to load swap: {}, continuing ...", e),
            };
        });
    }
    Ok(())
}
