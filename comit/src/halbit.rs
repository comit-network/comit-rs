use crate::{asset, identity, RelativeTime, Secret, SecretHash};
use bitcoin::hashes::core::fmt::Formatter;
use futures::{future, future::Either, Stream, TryFutureExt};
use genawaiter::sync::Gen;
use std::fmt;

/// Creates a new instance of the halbit protocol.
///
/// Returns a stream of events happening during the execution.
pub fn new<'a, C>(
    connector: &'a C,
    params: Params,
) -> impl Stream<Item = anyhow::Result<Event>> + 'a
where
    C: WaitForOpened + WaitForAccepted + WaitForSettled + WaitForCancelled,
{
    Gen::new({
        |co| async move {
            co.yield_(Ok(Event::Started)).await;

            let opened_or_error = connector
                .wait_for_opened(&params)
                .map_ok(Event::Opened)
                .await;
            co.yield_(opened_or_error).await;

            let accepted_or_error = connector
                .wait_for_accepted(&params)
                .map_ok(Event::Accepted)
                .await;
            co.yield_(accepted_or_error).await;

            let settled = connector.wait_for_settled(&params);
            let cancelled = connector.wait_for_cancelled(&params);

            match future::try_select(settled, cancelled).await {
                Ok(Either::Left((settled, _))) => {
                    co.yield_(Ok(Event::Settled(settled))).await;
                }
                Ok(Either::Right((cancelled, _))) => {
                    co.yield_(Ok(Event::Cancelled(cancelled))).await;
                }
                Err(either) => {
                    let (error, _other_future) = either.factor_first();

                    co.yield_(Err(error)).await;
                }
            }
        }
    })
}

#[derive(Copy, Clone, Debug)]
pub struct Params {
    pub redeem_identity: identity::Lightning,
    pub refund_identity: identity::Lightning,
    pub cltv_expiry: RelativeTime,
    pub asset: asset::Bitcoin,
    pub secret_hash: SecretHash,
}

/// Resolves when said event has occured.
#[async_trait::async_trait]
pub trait WaitForOpened {
    async fn wait_for_opened(&self, params: &Params) -> anyhow::Result<Opened>;
}

#[async_trait::async_trait]
pub trait WaitForAccepted {
    async fn wait_for_accepted(&self, params: &Params) -> anyhow::Result<Accepted>;
}

#[async_trait::async_trait]
pub trait WaitForSettled {
    async fn wait_for_settled(&self, params: &Params) -> anyhow::Result<Settled>;
}

#[async_trait::async_trait]
pub trait WaitForCancelled {
    async fn wait_for_cancelled(&self, params: &Params) -> anyhow::Result<Cancelled>;
}

/// Represents the events in the halbit protocol.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// The halbit protocol was started.
    Started,

    /// The invoice was opened and is ready to accept a payment.
    ///
    /// On the recipient side, this means the hold invoice has been added to
    /// lnd. On the (payment) sender side, we cannot (yet) know about this
    /// so we just have to assume that this happens.
    Opened(Opened),

    /// The payment to the invoice was accepted but the preimage has not yet
    /// been revealed.
    ///
    /// On the recipient side, this means the hold invoice moved to the
    /// `Accepted` state. On the (payment) sender side, we assume that once
    /// the payment is `InFlight`, it also reached the recipient.
    Accepted(Accepted),

    /// The payment is settled and therefore the preimage was revealed.
    Settled(Settled),

    /// The payment was cancelled.
    Cancelled(Cancelled),
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Event::Started => "Started",
            Event::Opened(_) => "Opened",
            Event::Accepted(_) => "Accepted",
            Event::Settled(_) => "Settled",
            Event::Cancelled(_) => "Cancelled",
        };

        write!(f, "{}", name)
    }
}

/// Represents the data available at said state.
///
/// These empty types are useful because they give us additional type safety.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opened;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Accepted;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settled {
    pub secret: Secret,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cancelled;
