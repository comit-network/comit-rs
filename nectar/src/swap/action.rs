use crate::{
    database::{Load, Save},
    SwapId,
};
use anyhow::Context;
use futures::{future::FutureExt, Future};

/// Try to do an action resulting in the event `E`.
///
/// If we can `Load` the event `E` corresponding to `swap_id` from the
/// `DB`, we early return `Ok(E)` early and do not try to do the
/// action again.
///
///
/// We `futures::future::select` on the `action` future and the
/// `poll_abortion_condition` future passed as arguments. If the
/// `action` is completed successfully before the
/// `poll_abortion_condition` future is met, we return `Ok(E)`.
/// Otherwise, we return `Err(AbortConditionMet)`.
///
/// Before returning the event resulted from successfully executing
/// the action, we store it in the `DB` through the `Save` trait.
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

    let event = futures::select! {
        event = action.fuse() => event.context("failed to execute action")?,
        abort = poll_abortion_condition.fuse() => {
            anyhow::bail!(abort
                          .map(|_| AbortConditionMet)
                          .context("error when polling abort condition")?)
        }
    };

    db.save(event.clone(), swap_id).await?;
    Ok(event)
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
    async fn trying_to_do_an_arbitrary_action_more_than_once_is_idempotent() {
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
