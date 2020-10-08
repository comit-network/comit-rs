use crate::swap::{
    comit::{SwapFailedNoRefund, SwapFailedShouldRefund},
    hbit, herc20,
};
use anyhow::{Context, Result};
use comit::Secret;
use genawaiter::sync::{Gen, GenBoxed};
use time::OffsetDateTime;

pub enum Action {
    ExecuteHbitFund(hbit::Params),
    ExecuteHerc20Redeem(herc20::Params, Secret, herc20::Deployed),
    ExecuteHbitRefund(hbit::Params, hbit::Funded),
}

pub enum Event {
    Herc20Deployed(herc20::Deployed),
    Herc20Funded(herc20::Funded),
    HbitFunded(hbit::Funded),
    HbitRedeemed(hbit::Redeemed),
    Herc20Redeemed(herc20::Redeemed),
}

pub enum Out {
    Event(Event),
    Action(Action),
}

/// Execute a Herc20<->Hbit swap for Bob.
pub fn herc20_hbit_bob<W>(
    world: W,
    herc20_params: herc20::Params,
    hbit_params: hbit::Params,
    utc_start_of_swap: OffsetDateTime,
) -> GenBoxed<Out, (), anyhow::Result<()>>
where
    W: hbit::WatchForFunded
        + hbit::WatchForRedeemed
        + herc20::WatchForDeployed
        + herc20::WatchForFunded
        + herc20::WatchForRedeemed
        + Send
        + Sync
        + 'static,
{
    Gen::new_boxed(|co| async move {
        tracing::info!("starting swap");

        let swap_result: Result<()> = async {
            let herc20_deployed = world
                .watch_for_deployed(herc20_params.clone(), utc_start_of_swap)
                .await
                .context(SwapFailedNoRefund)?;

            tracing::info!("alice deployed the herc20 htlc");
            co.yield_(Out::Event(Event::Herc20Deployed(herc20_deployed.clone())))
                .await;

            let herc20_funded = herc20::WatchForFunded::watch_for_funded(
                &world,
                herc20_params.clone(),
                herc20_deployed.clone(),
                utc_start_of_swap,
            )
            .await
            .context(SwapFailedNoRefund)?;

            tracing::info!("alice funded the herc20 htlc");
            co.yield_(Out::Event(Event::Herc20Funded(herc20_funded.clone())))
                .await;
            co.yield_(Out::Action(Action::ExecuteHbitFund(hbit_params)))
                .await;

            let hbit_funded =
                hbit::WatchForFunded::watch_for_funded(&world, &hbit_params, utc_start_of_swap)
                    .await
                    .context(SwapFailedNoRefund)?;

            tracing::info!("we funded the hbit htlc");
            co.yield_(Out::Event(Event::HbitFunded(hbit_funded))).await;

            let hbit_redeemed = hbit::WatchForRedeemed::watch_for_redeemed(
                &world,
                &hbit_params,
                hbit_funded,
                utc_start_of_swap,
            )
            .await
            .context(SwapFailedShouldRefund(hbit_funded))?;

            tracing::info!("alice redeemed the hbit htlc");
            co.yield_(Out::Event(Event::HbitRedeemed(hbit_redeemed.clone())))
                .await;

            co.yield_(Out::Action(Action::ExecuteHerc20Redeem(
                herc20_params,
                hbit_redeemed.secret,
                herc20_deployed.clone(),
            )))
            .await;

            let herc20_redeemed = herc20::WatchForRedeemed::watch_for_redeemed(
                &world,
                herc20_deployed.clone(),
                utc_start_of_swap,
            )
            .await
            .context(SwapFailedShouldRefund(hbit_funded))?;

            tracing::info!("we redeemed the herc20 htlc");
            co.yield_(Out::Event(Event::Herc20Redeemed(herc20_redeemed.clone())))
                .await;

            Ok(())
        }
        .await;

        if let Err(e) = swap_result {
            if let Some(error) = e.downcast_ref::<SwapFailedShouldRefund<hbit::Funded>>() {
                co.yield_(Out::Action(Action::ExecuteHbitRefund(hbit_params, error.0)))
                    .await;
            }

            return Err(e);
        }

        Ok(())
    })
}
