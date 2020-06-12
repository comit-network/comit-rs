use crate::{
    db::AcceptedSwap,
    swap_protocols::rfc003::{
        create_swap::{create_watcher, OngoingSwap},
        events::{HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded},
        state::Insert,
        Accept, Request, SwapCommunication,
    },
};
use tracing_futures::Instrument;

#[allow(clippy::cognitive_complexity)]
pub async fn init_accepted_swap<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>(
    dependencies: &Rfc003Facade,
    accepted: AcceptedSwap<AL, BL, AA, BA, AI, BI>,
) -> anyhow::Result<()>
where
    Rfc003Facade: HtlcFunded<AL, AA, AH, AI, AT>
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
    let (request, accept, accepted_at) = accepted;
    let id = request.swap_id;

    dependencies
        .insert(id, SwapCommunication::Accepted {
            request: request.clone(),
            response: accept,
        })
        .await;

    let swap = OngoingSwap::new(request, accept);

    tracing::trace!("initialising accepted swap: {}", id);

    tokio::task::spawn(
        create_watcher::<_, _, _, _, AH, _, AT>(
            dependencies.clone(),
            dependencies.alpha_ledger_states.clone(),
            id,
            swap.alpha_htlc_params(),
            accepted_at,
        )
        .instrument(tracing::info_span!("alpha")),
    );

    tokio::task::spawn(
        create_watcher::<_, _, _, _, BH, _, BT>(
            dependencies.clone(),
            dependencies.beta_ledger_states.clone(),
            id,
            swap.beta_htlc_params(),
            accepted_at,
        )
        .instrument(tracing::info_span!("beta")),
    );

    Ok(())
}
