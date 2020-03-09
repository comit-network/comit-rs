use crate::{
    db::AcceptedSwap,
    seed::DeriveSwapSeed,
    swap_protocols::{
        rfc003::{
            alice, bob, create_alpha_watcher,
            create_swap::{create_beta_watcher, SwapEvent},
            events::{HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded},
            Accept, Request,
        },
        state_store::StateStore,
        Role,
    },
};

#[allow(clippy::cognitive_complexity)]
pub fn init_accepted_swap<D, AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>(
    dependencies: &D,
    accepted: AcceptedSwap<AL, BL, AA, BA, AI, BI>,
    role: Role,
) -> anyhow::Result<()>
where
    D: StateStore<
            alice::State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>,
            SwapEvent<AA, BA, AH, BH, AT, BT>,
        > + StateStore<
            bob::State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>,
            SwapEvent<AA, BA, AH, BH, AT, BT>,
        > + Clone
        + DeriveSwapSeed
        + HtlcFunded<AL, AA, AH, AI, AT>
        + HtlcFunded<BL, BA, BH, BI, BT>
        + HtlcDeployed<AL, AA, AH, AI, AT>
        + HtlcDeployed<BL, BA, BH, BI, BT>
        + HtlcRedeemed<AL, AA, AH, AI, AT>
        + HtlcRedeemed<BL, BA, BH, BI, BT>
        + HtlcRefunded<AL, AA, AH, AI, AT>
        + HtlcRefunded<BL, BA, BH, BI, BT>,
    AL: Clone + Send + Sync + 'static,
    BL: Clone + Send + Sync + 'static,
    AA: Ord + Clone + Send + Sync + 'static,
    BA: Ord + Clone + Send + Sync + 'static,
    AH: Clone + Send + Sync + 'static,
    BH: Clone + Send + Sync + 'static,
    AI: Clone + Send + Sync + 'static,
    BI: Clone + Send + Sync + 'static,
    AT: Clone + Send + Sync + 'static,
    BT: Clone + Send + Sync + 'static,
    Request<AL, BL, AA, BA, AI, BI>: Clone,
    Accept<AI, BI>: Copy,
{
    let (request, accept, _) = &accepted;

    let id = request.swap_id;
    let seed = dependencies.derive_swap_seed(id);
    tracing::trace!("initialising accepted swap: {}", id);

    match role {
        Role::Alice => {
            let state = alice::State::accepted(request.clone(), *accept, seed);
            StateStore::<_, SwapEvent<AA, BA, AH, BH, AT, BT>>::insert(dependencies, id, state);

            tokio::task::spawn(create_alpha_watcher::<
                D,
                alice::State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>,
                AI,
                BI,
            >(dependencies.clone(), accepted.clone()));

            tokio::task::spawn(create_beta_watcher::<
                D,
                alice::State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>,
                AI,
                BI,
            >(dependencies.clone(), accepted));
        }
        Role::Bob => {
            let state: bob::State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT> =
                bob::State::accepted(request.clone(), *accept, seed);
            StateStore::insert(dependencies, id, state);

            tokio::task::spawn(create_alpha_watcher::<
                D,
                bob::State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>,
                AI,
                BI,
            >(dependencies.clone(), accepted.clone()));

            tokio::task::spawn(create_beta_watcher::<
                D,
                bob::State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>,
                AI,
                BI,
            >(dependencies.clone(), accepted));
        }
    };

    Ok(())
}
