use super::*;

#[allow(clippy::type_complexity)]
async fn insert_state_for_bob<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT, DB>(
    db: DB,
    swap_communication_states: Arc<SwapCommunicationStates>,
    alpha_ledger_state: Arc<LedgerStates>,
    beta_ledger_state: Arc<LedgerStates>,
    counterparty: PeerId,
    swap_request: Request<AL, BL, AA, BA, AI, BI>,
) -> anyhow::Result<()>
where
    AL: Send + 'static,
    BL: Send + 'static,
    AA: Ord + Send + 'static,
    BA: Ord + Send + 'static,
    AH: Send + 'static,
    BH: Send + 'static,
    AI: Send + 'static,
    BI: Send + 'static,
    AT: Send + 'static,
    BT: Send + 'static,
    DB: Save<Request<AL, BL, AA, BA, AI, BI>> + Save<Swap>,
    Request<AL, BL, AA, BA, AI, BI>: Clone,
{
    let id = swap_request.swap_id;

    Save::save(&db, Swap::new(id, Role::Bob, counterparty)).await?;
    Save::save(&db, swap_request.clone()).await?;

    swap_communication_states
        .insert(id, SwapCommunication::Proposed {
            request: swap_request,
        })
        .await;

    alpha_ledger_state
        .insert(id, LedgerState::<AA, AH, AT>::NotDeployed)
        .await;
    beta_ledger_state
        .insert(id, LedgerState::<BA, BH, BT>::NotDeployed)
        .await;

    Ok(())
}
