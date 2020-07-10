use crate::{swap::db, SwapId};
use comit::Timestamp;

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

        if self.should_abort().await? {
            return Ok(Next::Abort);
        }

        let event = Execute::<E>::execute(self, execution_args).await?;
        self.remember(event.clone()).await?;

        Ok(Next::Continue(event))
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

#[async_trait::async_trait]
pub trait CheckMemory<E> {
    async fn check_memory(&self) -> anyhow::Result<Option<E>>;
}

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

#[async_trait::async_trait]
pub trait Execute<E> {
    type Args;
    async fn execute(&self, args: Self::Args) -> anyhow::Result<E>;
}

#[async_trait::async_trait]
pub trait Remember<E> {
    async fn remember(&self, event: E) -> anyhow::Result<()>;
}

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

#[derive(Debug, Clone, Copy)]
pub enum Next<E> {
    Continue(E),
    Abort,
}

pub trait BetaExpiry {
    fn beta_expiry(&self) -> Timestamp;
}

#[async_trait::async_trait]
pub trait AlphaLedgerTime {
    async fn alpha_ledger_time(&self) -> anyhow::Result<Timestamp>;
}

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
