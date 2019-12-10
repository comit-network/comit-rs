use crate::{
    seed::SwapSeed,
    swap_protocols::{
        asset::Asset,
        rfc003::{
            alice, bob,
            events::HtlcEvents,
            state_machine::{self, SwapStates},
            state_store::StateStore,
            Accept, Ledger, Request,
        },
        Role, SwapId,
    },
};
use futures::{Future, Stream};
use tokio::executor::Executor;

#[allow(clippy::cognitive_complexity)]
pub fn init_accepted_swap<D, AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
    dependencies: &D,
    request: Request<AL, BL, AA, BA>,
    accept: Accept<AL, BL>,
    role: Role,
) -> anyhow::Result<()>
where
    D: StateStore + Clone + SwapSeed + Executor + HtlcEvents<AL, AA> + HtlcEvents<BL, BA>,
{
    let id = request.swap_id;
    let seed = SwapSeed::swap_seed(dependencies, id);

    match role {
        Role::Alice => {
            let state = alice::State::accepted(request.clone(), accept, seed);
            StateStore::insert(dependencies, id, state.clone());
        }
        Role::Bob => {
            let state = bob::State::accepted(request.clone(), accept, seed);
            StateStore::insert(dependencies, id, state.clone());
        }
    };

    let (swap_execution, receiver) =
        state_machine::create_swap(dependencies.clone(), request, accept);

    spawn(dependencies, id, swap_execution, receiver, role)
}

fn spawn<D, AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
    dependencies: &D,
    id: SwapId,
    swap_execution: impl Future<Item = (), Error = ()> + Send + 'static,
    receiver: impl Stream<Item = SwapStates<AL, BL, AA, BA>, Error = ()> + Send + 'static,
    role: Role,
) -> anyhow::Result<()>
where
    D: Executor + StateStore + Clone,
{
    let mut dependencies = dependencies.clone();

    dependencies.spawn(Box::new(swap_execution))?;

    dependencies.spawn(Box::new(receiver.for_each({
        let dependencies = dependencies.clone();
        move |update| {
            match role {
                Role::Alice => {
                    StateStore::update::<alice::State<AL, BL, AA, BA>>(&dependencies, &id, update)
                }
                Role::Bob => {
                    StateStore::update::<bob::State<AL, BL, AA, BA>>(&dependencies, &id, update)
                }
            }
            Ok(())
        }
    })))?;
    Ok(())
}
