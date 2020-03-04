use crate::{
    db::AcceptedSwap,
    seed::DeriveSwapSeed,
    swap_protocols::{
        rfc003::{
            alice, bob, create_swap,
            events::{HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded},
            state_store::StateStore,
            Accept, Ledger, Request,
        },
        Role,
    },
};

#[allow(clippy::cognitive_complexity)]
pub fn init_accepted_swap<D, AL, BL, AA, BA, AH, BH, AI, BI>(
    dependencies: &D,
    accepted: AcceptedSwap<AL, BL, AA, BA, AI, BI>,
    role: Role,
) -> anyhow::Result<()>
where
    D: StateStore
        + Clone
        + DeriveSwapSeed
        + HtlcFunded<AL, AA, AH, AI>
        + HtlcFunded<BL, BA, BH, BI>
        + HtlcDeployed<AL, AA, AH, AI>
        + HtlcDeployed<BL, BA, BH, BI>
        + HtlcRedeemed<AL, AA, AH, AI>
        + HtlcRedeemed<BL, BA, BH, BI>
        + HtlcRefunded<AL, AA, AH, AI>
        + HtlcRefunded<BL, BA, BH, BI>,
    AL: Ledger,
    BL: Ledger,
    AA: Ord + Clone + Send + Sync + 'static,
    BA: Ord + Clone + Send + Sync + 'static,
    AH: Clone + Send + Sync + 'static,
    BH: Clone + Send + Sync + 'static,
    AI: Clone + Send + Sync + 'static,
    BI: Clone + Send + Sync + 'static,
    Request<AL, BL, AA, BA, AI, BI>: Clone,
    Accept<AI, BI>: Copy,
{
    let (request, accept, _) = &accepted;

    let id = request.swap_id;
    let seed = dependencies.derive_swap_seed(id);
    tracing::trace!("initialising accepted swap: {}", id);

    match role {
        Role::Alice => {
            let state: alice::State<AL, BL, AA, BA, AH, BH, AI, BI> =
                alice::State::accepted(request.clone(), *accept, seed);
            StateStore::insert(dependencies, id, state);

            tokio::task::spawn(create_swap::<
                D,
                alice::State<AL, BL, AA, BA, AH, BH, AI, BI>,
                AI,
                BI,
            >(dependencies.clone(), accepted));
        }
        Role::Bob => {
            let state: bob::State<AL, BL, AA, BA, AH, BH, AI, BI> =
                bob::State::accepted(request.clone(), *accept, seed);
            StateStore::insert(dependencies, id, state);

            tokio::task::spawn(create_swap::<
                D,
                bob::State<AL, BL, AA, BA, AH, BH, AI, BI>,
                AI,
                BI,
            >(dependencies.clone(), accepted));
        }
    };

    Ok(())
}
