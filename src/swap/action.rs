use crate::{
    swap::db::{Load, Save},
    SwapId,
};
use comit::Timestamp;
use futures::{
    future::{self, Either},
    pin_mut, Future,
};

/// Try to do an action resulting in the event `E`.
///
/// If we already know that event `E` has happened because we can look
/// it up in our internal state via `LookUpEvent`, we return
/// `Next::Continue<E>` early and do not try to do the action again.
///
/// We do the action by calling `Execute::<E>::execute` and awaiting
/// on it, which will yield the event `E` if it resolves successfully.
///
/// Whilst waiting for the `Execute<E>` future to resolve, we
/// continuously check if `BetaHasExpired`. If Beta expires before
/// we finish doing the action, we fail.
///
/// If doing the action succeeds, we store the resulting event `E` via
/// `StoreEvent` to ensure that repeated calls to this function do not
/// result in doing the action more than once.
#[allow(dead_code)] // Not sure why this is flagged as dead code
pub async fn try_do_it_once<E, DB>(
    db: &DB,
    swap_id: SwapId,
    action: impl Future<Output = anyhow::Result<E>>,
    poll_abortion_condition: impl Future<Output = anyhow::Result<()>>,
) -> anyhow::Result<E>
where
    DB: Load<E> + Save<E>,
    E: Clone + Send + Sync + 'static,
{
    if let Some(event) = db.load(swap_id)? {
        return Ok(event);
    }

    pin_mut!(action);
    pin_mut!(poll_abortion_condition);

    let event = match future::select(action, poll_abortion_condition).await {
        Either::Left((Ok(event), _)) => event,
        Either::Right(_) => anyhow::bail!(AbortConditionMet),
        _ => anyhow::bail!("A future has failed"),
    };

    db.save(event.clone(), swap_id).await?;
    Ok(event)
}

/// Fetch the current `Timestamp` for the a ledger.
#[async_trait::async_trait]
pub trait LedgerTime {
    async fn ledger_time(&self) -> anyhow::Result<Timestamp>;
}

#[allow(dead_code)] // Not sure why this is flagged as dead code
pub async fn poll_beta_has_expired<BC>(
    beta_connector: &BC,
    beta_expiry: Timestamp,
) -> anyhow::Result<()>
where
    BC: LedgerTime,
{
    loop {
        let beta_ledger_time = beta_connector.ledger_time().await?;

        if beta_expiry <= beta_ledger_time {
            return Ok(());
        }

        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
    }
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("The abort condition has been met")]
pub struct AbortConditionMet;

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
    };

    struct FakeActor {
        wallet: FakeWallet,
    }

    #[derive(Default)]
    struct FakeDatabase {
        events: Arc<RwLock<HashMap<SwapId, ArbitraryEvent>>>,
    }

    impl FakeActor {
        async fn arbitrary_action(&self) -> anyhow::Result<ArbitraryEvent> {
            let mut blockchain = self.wallet.node.write().unwrap();
            blockchain.events.push(ArbitraryEvent);

            Ok(ArbitraryEvent)
        }
    }

    struct FakeWallet {
        node: Arc<RwLock<FakeBlockchain>>,
    }

    #[derive(Default)]
    struct FakeBlockchain {
        events: Vec<ArbitraryEvent>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ArbitraryEvent;

    #[async_trait::async_trait]
    impl Save<ArbitraryEvent> for FakeDatabase {
        async fn save(&self, deploy_event: ArbitraryEvent, swap_id: SwapId) -> anyhow::Result<()> {
            let mut events = self.events.write().unwrap();
            events.insert(swap_id, deploy_event);

            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Load<ArbitraryEvent> for FakeDatabase {
        fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<ArbitraryEvent>> {
            let events = self.events.read().unwrap();

            Ok(events.get(&swap_id).cloned())
        }
    }

    #[tokio::test]
    async fn trying_to_do_an_arbitrary_action_once_is_idempotent() {
        let blockchain = Arc::new(RwLock::new(FakeBlockchain::default()));
        let wallet = FakeWallet {
            node: Arc::clone(&blockchain),
        };

        let db = FakeDatabase::default();

        let swap_id = SwapId::default();

        let actor = FakeActor { wallet };

        assert!(blockchain.read().unwrap().events.is_empty());
        let res = try_do_it_once(
            &db,
            swap_id,
            actor.arbitrary_action(),
            futures::future::pending(),
        )
        .await;

        assert!(matches!(res, Ok(ArbitraryEvent)));
        assert_eq!(blockchain.read().unwrap().events.len(), 1);

        let res = try_do_it_once(
            &db,
            swap_id,
            actor.arbitrary_action(),
            futures::future::pending(),
        )
        .await;
        assert!(matches!(res, Ok(ArbitraryEvent)));
        assert_eq!(blockchain.read().unwrap().events.len(), 1);
    }
}
