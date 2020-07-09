use crate::{swap::db, SwapId};
use comit::Timestamp;
use futures::{
    future::{self, Either},
    pin_mut,
};
use std::time::Duration;

/// Try to do an action resulting in the event `E`.
///
/// If we already know about the event `E` because it's part of our
/// "Memory", we return `Next::Continue` early and do not try to do
/// the action again.
///
/// We do the action by calling `Execute::<E>::execute` and awaiting
/// on it, which will yield the event `E` if it resolves successfully.
///
/// Whilst waiting for the `Execute<E>` future to resolve, we
/// continuously check if we `ShouldAbort`. If we `ShouldAbort` before
/// we finish doing the action, we return `Next::Abort`.
///
/// If doing the action succeeds, we `Remember` the resulting event
/// `E` so that repeated calls to this function do not result in doing
/// the action more than once.
#[async_trait::async_trait]
pub trait TryDoItOnce<E>
where
    Self: CheckMemory<E> + ShouldAbort + Execute<E> + Remember<E>,
    E: Clone + Send + Sync + 'static,
    <Self as Execute<E>>::Args: Send + Sync,
{
    async fn try_do_it_once(
        &self,
        execution_args: <Self as Execute<E>>::Args,
    ) -> anyhow::Result<Next<E>> {
        if let Some(event) = self.check_memory().await? {
            return Ok(Next::Continue(event));
        }

        let should_abort = async {
            loop {
                if self.should_abort().await.unwrap_or(true) {
                    return;
                }

                tokio::time::delay_for(Duration::from_secs(1)).await;
            }
        };
        let execute_future = Execute::<E>::execute(self, execution_args);

        pin_mut!(execute_future);
        pin_mut!(should_abort);

        match future::select(execute_future, should_abort).await {
            Either::Left((Ok(event), _)) => {
                self.remember(event.clone()).await?;
                Ok(Next::Continue(event))
            }
            _ => Ok(Next::Abort),
        }
    }
}

#[async_trait::async_trait]
impl<E, A> TryDoItOnce<E> for A
where
    A: CheckMemory<E> + ShouldAbort + Execute<E> + Remember<E>,
    E: Clone + Send + Sync + 'static,
    <Self as Execute<E>>::Args: Send + Sync,
{
}

/// Do an action resulting in the event `E`.
///
/// If we already know about the event `E` because it's part of our
/// "Memory", we return `Next::Continue` early and do not do the
/// action again.
///
/// We do the action by calling `Execute::<E>::execute` and awaiting
/// on it, which will yield the event `E` if it resolves successfully.
///
/// If doing the action succeeds, we `Remember` the resulting event
/// `E` so that repeated calls to this function do not result in doing
/// the action more than once.
#[async_trait::async_trait]
pub trait DoItOnce<E>
where
    Self: CheckMemory<E> + Execute<E> + Remember<E>,
    E: Clone + Send + Sync + 'static,
    <Self as Execute<E>>::Args: Send + Sync,
{
    async fn do_it_once(&self, execution_args: <Self as Execute<E>>::Args) -> anyhow::Result<E> {
        if let Some(event) = self.check_memory().await? {
            return Ok(event);
        }

        let event = Execute::<E>::execute(self, execution_args).await?;
        self.remember(event.clone()).await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl<E, A> DoItOnce<E> for A
where
    A: CheckMemory<E> + ShouldAbort + Execute<E> + Remember<E>,
    E: Clone + Send + Sync + 'static,
    <Self as Execute<E>>::Args: Send + Sync,
{
}

/// Look for the event `E` in our "Memory".
#[async_trait::async_trait]
pub trait CheckMemory<E> {
    async fn check_memory(&self) -> anyhow::Result<Option<E>>;
}

/// Look for the event `E` by attempting to `Load` it from a database.
#[async_trait::async_trait]
impl<E, A> CheckMemory<E> for A
where
    A: db::Load<E> + std::ops::Deref<Target = SwapId>,
    E: 'static,
{
    async fn check_memory(&self) -> anyhow::Result<Option<E>> {
        self.load(**self).await
    }
}

/// Determine if we should abort before `Do`-ing an action.
#[async_trait::async_trait]
pub trait ShouldAbort {
    async fn should_abort(&self) -> anyhow::Result<bool>;
}

/// For Nectar, we should abort if Beta has expired, meaning that Beta
/// ledger has reached or surpassed the expiry time of the Beta
/// contract.
#[async_trait::async_trait]
impl<A> ShouldAbort for A
where
    A: BetaLedgerTime + BetaExpiry + Sync,
{
    async fn should_abort(&self) -> anyhow::Result<bool> {
        let beta_ledger_time = self.beta_ledger_time().await?;

        Ok(self.beta_expiry() <= beta_ledger_time)
    }
}

/// Execute an action which yields the event `E`.
#[async_trait::async_trait]
pub trait Execute<E> {
    type Args;
    async fn execute(&self, args: Self::Args) -> anyhow::Result<E>;
}

/// Add the event `E` to our "Memory", so as to `Remember` it.
#[async_trait::async_trait]
pub trait Remember<E> {
    async fn remember(&self, event: E) -> anyhow::Result<()>;
}

/// Add the event `E` to our "Memory", by saving it to a database.
#[async_trait::async_trait]
impl<E, A> Remember<E> for A
where
    A: db::Save<E> + std::ops::Deref<Target = SwapId>,
    E: Send + 'static,
{
    async fn remember(&self, event: E) -> anyhow::Result<()> {
        self.save(event, **self).await
    }
}

/// Result of doing a conditional protocol action.
///
/// If the action was done successfully we `Continue` and obtain the
/// event `E`. Otherwise we `Abort`.
#[derive(Debug, Clone, Copy)]
pub enum Next<E> {
    Continue(E),
    Abort,
}

/// Get the expiry timestamp for the Beta asset in a swap protocol.
pub trait BetaExpiry {
    fn beta_expiry(&self) -> Timestamp;
}

/// Fetch the current `Timestamp` for the Beta ledger in a swap
/// protocol.
#[async_trait::async_trait]
pub trait BetaLedgerTime {
    async fn beta_ledger_time(&self) -> anyhow::Result<Timestamp>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swap::{Next, TryDoItOnce};
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
    };

    struct FakeActor {
        db: FakeDatabase,
        wallet: FakeWallet,
        swap_id: SwapId,
    }

    #[derive(Default)]
    struct FakeDatabase {
        events: Arc<RwLock<HashMap<SwapId, ArbitraryEvent>>>,
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

    #[allow(clippy::unit_arg)]
    #[async_trait::async_trait]
    impl Execute<ArbitraryEvent> for FakeActor {
        type Args = ();
        async fn execute(&self, (): Self::Args) -> anyhow::Result<ArbitraryEvent> {
            let mut blockchain = self.wallet.node.write().unwrap();
            blockchain.events.push(ArbitraryEvent);

            Ok(ArbitraryEvent)
        }
    }

    #[async_trait::async_trait]
    impl ShouldAbort for FakeActor {
        async fn should_abort(&self) -> anyhow::Result<bool> {
            Ok(false)
        }
    }

    #[async_trait::async_trait]
    impl db::Load<ArbitraryEvent> for FakeActor {
        async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<ArbitraryEvent>> {
            let events = self.db.events.read().unwrap();

            Ok(events.get(&swap_id).cloned())
        }
    }

    #[async_trait::async_trait]
    impl db::Save<ArbitraryEvent> for FakeActor {
        async fn save(&self, deploy_event: ArbitraryEvent, swap_id: SwapId) -> anyhow::Result<()> {
            let mut events = self.db.events.write().unwrap();
            events.insert(swap_id, deploy_event);

            Ok(())
        }
    }

    impl std::ops::Deref for FakeActor {
        type Target = SwapId;
        fn deref(&self) -> &Self::Target {
            &self.swap_id
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

        let actor = FakeActor {
            db,
            wallet,
            swap_id,
        };

        assert!(blockchain.read().unwrap().events.is_empty());
        let res = actor.try_do_it_once(()).await;

        assert!(matches!(res, Ok(Next::Continue(ArbitraryEvent))));
        assert_eq!(blockchain.read().unwrap().events.len(), 1);

        let res = actor.try_do_it_once(()).await;
        assert!(matches!(res, Ok(Next::Continue(ArbitraryEvent))));
        assert_eq!(blockchain.read().unwrap().events.len(), 1);
    }

    #[tokio::test]
    async fn doing_an_arbitrary_action_once_is_idempotent() {
        let blockchain = Arc::new(RwLock::new(FakeBlockchain::default()));
        let wallet = FakeWallet {
            node: Arc::clone(&blockchain),
        };

        let db = FakeDatabase::default();

        let swap_id = SwapId::default();

        let actor = FakeActor {
            db,
            wallet,
            swap_id,
        };

        assert!(blockchain.read().unwrap().events.is_empty());
        let res = actor.do_it_once(()).await;

        assert!(matches!(res, Ok(ArbitraryEvent)));
        assert_eq!(blockchain.read().unwrap().events.len(), 1);

        let res = actor.do_it_once(()).await;
        assert!(matches!(res, Ok(ArbitraryEvent)));
        assert_eq!(blockchain.read().unwrap().events.len(), 1);
    }
}
