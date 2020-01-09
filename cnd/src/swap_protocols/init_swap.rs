use crate::{
    seed::DeriveSwapSeed,
    swap_protocols::{
        asset::Asset,
        rfc003::{
            alice, bob, create_swap, events::HtlcEvents, state_store::StateStore, Accept, Ledger,
            Request,
        },
        Role,
    },
};
use futures_core::{FutureExt, TryFutureExt};
use tokio_executor01::Executor;

#[allow(clippy::cognitive_complexity)]
pub fn init_accepted_swap<D, AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
    dependencies: &D,
    request: Request<AL, BL, AA, BA>,
    accept: Accept<AL, BL>,
    role: Role,
) -> anyhow::Result<()>
where
    D: StateStore + Clone + DeriveSwapSeed + Executor + HtlcEvents<AL, AA> + HtlcEvents<BL, BA>,
{
    let id = request.swap_id;
    let seed = dependencies.derive_swap_seed(id);

    match role {
        Role::Alice => {
            let state = alice::State::accepted(request.clone(), accept, seed);
            StateStore::insert(dependencies, id, state);

            let swap_execution = create_swap::<D, alice::State<AL, BL, AA, BA>>(
                dependencies.clone(),
                request,
                accept,
            );

            dependencies
                .clone()
                .spawn(Box::new(swap_execution.unit_error().boxed().compat()))?;
        }
        Role::Bob => {
            let state = bob::State::accepted(request.clone(), accept, seed);
            StateStore::insert(dependencies, id, state);

            let swap_execution =
                create_swap::<D, bob::State<AL, BL, AA, BA>>(dependencies.clone(), request, accept);

            dependencies
                .clone()
                .spawn(Box::new(swap_execution.unit_error().boxed().compat()))?;
        }
    };

    Ok(())
}
