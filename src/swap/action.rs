use crate::{swap::db, SwapId};
use comit::Timestamp;
use futures::{
    future::{self, Either},
    pin_mut,
};
use std::time::Duration;

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
#[async_trait::async_trait]
pub trait TryDoItOnce<E>
where
    Self: LookUpEvent<E> + BetaHasExpired + Execute<E> + StoreEvent<E>,
    E: Clone + Send + Sync + 'static,
    <Self as Execute<E>>::Args: Send + Sync,
{
    async fn try_do_it_once(
        &self,
        execution_args: <Self as Execute<E>>::Args,
    ) -> anyhow::Result<E> {
        if let Some(event) = self.look_up_event()? {
            return Ok(event);
        }

        // For Nectar, we conservatively abort if Beta has expired
        let beta_expired = async {
            loop {
                if self.beta_has_expired().await? {
                    return Result::<(), anyhow::Error>::Ok(());
                }

                tokio::time::delay_for(Duration::from_secs(1)).await;
            }
        };
        let execute_future = Execute::<E>::execute(self, execution_args);

        pin_mut!(execute_future);
        pin_mut!(beta_expired);

        match future::select(execute_future, beta_expired).await {
            Either::Left((Ok(event), _)) => {
                self.store_event(event.clone())?;
                Ok(event)
            }
            Either::Right(_) => anyhow::bail!(BetaHasExpiredError),
            _ => anyhow::bail!("A future has failed"),
        }
    }
}

#[async_trait::async_trait]
impl<E, A> TryDoItOnce<E> for A
where
    A: LookUpEvent<E> + BetaHasExpired + Execute<E> + StoreEvent<E>,
    E: Clone + Send + Sync + 'static,
    <Self as Execute<E>>::Args: Send + Sync,
{
}

/// Do an action resulting in the event `E`.
///
/// If we already know that event `E` has happened because we can look
/// it up in our internal state via `LookUpEvent`, we return
/// `Next::Continue<E>` early and do not try to do the action again.
///
/// We do the action by calling `Execute::<E>::execute` and awaiting
/// on it, which will yield the event `E` if it resolves successfully.
///
/// If doing the action succeeds, we store the resulting event `E` via
/// `StoreEvent` to ensure that repeated calls to this function do not
/// result in doing the action more than once.
#[async_trait::async_trait]
pub trait DoItOnce<E>
where
    Self: LookUpEvent<E> + Execute<E> + StoreEvent<E>,
    E: Clone + Send + Sync + 'static,
    <Self as Execute<E>>::Args: Send + Sync,
{
    async fn do_it_once(&self, execution_args: <Self as Execute<E>>::Args) -> anyhow::Result<E> {
        if let Some(event) = self.look_up_event()? {
            return Ok(event);
        }

        let event = Execute::<E>::execute(self, execution_args).await?;
        self.store_event(event.clone())?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl<E, A> DoItOnce<E> for A
where
    A: LookUpEvent<E> + BetaHasExpired + Execute<E> + StoreEvent<E>,
    E: Clone + Send + Sync + 'static,
    <Self as Execute<E>>::Args: Send + Sync,
{
}

pub trait LookUpEvent<E> {
    fn look_up_event(&self) -> anyhow::Result<Option<E>>;
}

/// Look up swap event `E` by attempting to `Load` it from a database
/// using our `SwapId`.
impl<E, A> LookUpEvent<E> for A
where
    A: db::Load<E> + AsSwapId,
    E: 'static,
{
    fn look_up_event(&self) -> anyhow::Result<Option<E>> {
        self.load(self.as_swap_id())
    }
}

#[async_trait::async_trait]
pub trait BetaHasExpired {
    async fn beta_has_expired(&self) -> anyhow::Result<bool>;
}

#[async_trait::async_trait]
impl<A> BetaHasExpired for A
where
    A: BetaLedgerTime + BetaExpiry + Sync,
{
    async fn beta_has_expired(&self) -> anyhow::Result<bool> {
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

pub trait StoreEvent<E> {
    fn store_event(&self, event: E) -> anyhow::Result<()>;
}

/// Store the event `E` associated with our `SwapId` by saving it to a
/// database through the `Save` trait.
impl<E, A> StoreEvent<E> for A
where
    A: db::Save<E> + AsSwapId,
    E: Send + 'static,
{
    fn store_event(&self, event: E) -> anyhow::Result<()> {
        self.save(event, self.as_swap_id())
    }
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

pub trait AsSwapId {
    fn as_swap_id(&self) -> SwapId;
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("Beta expiry has been reached")]
pub struct BetaHasExpiredError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swap::TryDoItOnce;
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
    impl BetaHasExpired for FakeActor {
        async fn beta_has_expired(&self) -> anyhow::Result<bool> {
            Ok(false)
        }
    }

    impl db::Load<ArbitraryEvent> for FakeActor {
        fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<ArbitraryEvent>> {
            let events = self.db.events.read().unwrap();

            Ok(events.get(&swap_id).cloned())
        }
    }

    impl db::Save<ArbitraryEvent> for FakeActor {
        fn save(&self, deploy_event: ArbitraryEvent, swap_id: SwapId) -> anyhow::Result<()> {
            let mut events = self.db.events.write().unwrap();
            events.insert(swap_id, deploy_event);

            Ok(())
        }
    }

    impl AsSwapId for FakeActor {
        fn as_swap_id(&self) -> SwapId {
            self.swap_id
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

        assert!(matches!(res, Ok(ArbitraryEvent)));
        assert_eq!(blockchain.read().unwrap().events.len(), 1);

        let res = actor.try_do_it_once(()).await;
        assert!(matches!(res, Ok(ArbitraryEvent)));
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
