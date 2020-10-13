use crate::{network::SwapDigest, SharedSwapId};
use futures::prelude::*;
use libp2p::{
    core::upgrade,
    request_response::{
        handler::RequestProtocol, ProtocolName, ProtocolSupport, RequestId, RequestResponse,
        RequestResponseCodec, RequestResponseConfig, RequestResponseEvent, RequestResponseMessage,
        ResponseChannel,
    },
    swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters},
    NetworkBehaviour, PeerId,
};
use serde::Deserialize;
use std::{
    collections::{HashMap, VecDeque},
    fmt, io,
    task::{Context, Poll},
    time::Duration,
};

/// Implements the Announce (/comit/swap/announce/1.0.0) protocol.
///
/// We don't implement any connection handling here but assume that another
/// network behaviour handles connections and peer-ID to address translation for
/// us.
///
/// The type parameter `C` represents the context that is used by the caller to
/// associate submitted announce requests with the events emitted by this
/// NetworkBehaviour.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourOutEvent<C>", poll_method = "poll")]
#[allow(missing_debug_implementations)]
pub struct Announce<C>
where
    C: fmt::Display + Send + 'static,
{
    inner: RequestResponse<AnnounceCodec>,

    #[behaviour(ignore)]
    awaiting_announcements: HashMap<SwapDigest, AwaitingAnnouncement<C>>,
    #[behaviour(ignore)]
    sent_announcements: HashMap<RequestId, C>,
    #[behaviour(ignore)]
    received_announcements: HashMap<SwapDigest, ReceivedAnnouncement>,
    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent<C>>,
}

#[derive(Debug)]
struct AwaitingAnnouncement<C> {
    /// The peer we are awaiting the announcement from.
    from: PeerId,
    /// The announce context provided by the user.
    context: C,
}

impl<C> AwaitingAnnouncement<C> {
    fn is_from(&self, peer: &PeerId) -> bool {
        &self.from == peer
    }
}

#[derive(Debug)]
struct ReceivedAnnouncement {
    /// The peer we received the announcement from.
    from: PeerId,
    /// The channel we can use to send a response back.
    channel: ResponseChannel<SharedSwapId>,
}

impl ReceivedAnnouncement {
    fn is_from(&self, peer: &PeerId) -> bool {
        &self.from == peer
    }
}

impl<C> Default for Announce<C>
where
    C: fmt::Display + Send + 'static,
{
    fn default() -> Self {
        Self::new(Duration::from_secs(5 * 60))
    }
}

impl<C> Announce<C>
where
    C: fmt::Display + Send + 'static,
{
    pub fn new(duration: Duration) -> Self {
        let mut config = RequestResponseConfig::default();
        config.set_request_timeout(duration);

        Self {
            inner: RequestResponse::new(
                AnnounceCodec::default(),
                vec![(AnnounceProtocol, ProtocolSupport::Full)],
                config,
            ),
            awaiting_announcements: Default::default(),
            sent_announcements: Default::default(),
            received_announcements: Default::default(),
            events: Default::default(),
        }
    }

    /// Announce a swap to the given peer.
    ///
    /// The context argument will be stored locally and passed back to the user
    /// in the emitted events and can therefore be used by the caller to
    /// distinguish concurrent announcements.
    #[tracing::instrument(level = "info", skip(self), fields(%context, %peer, %swap))]
    pub fn announce_swap(&mut self, swap: SwapDigest, peer: PeerId, context: C) {
        let request_id = self.inner.send_request(&peer, swap);

        tracing::info!("sending announcement with {:?}", request_id);

        self.sent_announcements.insert(request_id, context);
    }

    /// Await an announcement from a given peer.
    ///
    /// The context argument will be stored locally and passed back to the user
    /// in the emitted events and can therefore be used by the caller to
    /// distinguish concurrent announcements.
    #[tracing::instrument(level = "info", skip(self), fields(%context, %peer, %swap))]
    pub fn await_announcement(&mut self, swap: SwapDigest, peer: PeerId, context: C) {
        match self.received_announcements.remove(&swap) {
            Some(received_announcement)
                if received_announcement.is_from(&peer)
                    && received_announcement.channel.is_open() =>
            {
                self.confirm(peer, context, received_announcement.channel);
            }
            Some(received_announcement) => {
                self.abort(peer, context, received_announcement.channel);
            }
            None => {
                tracing::info!("no pending announcement");

                self.awaiting_announcements
                    .insert(swap, AwaitingAnnouncement {
                        from: peer,
                        context,
                    });
            }
        }
    }

    fn confirm(&mut self, peer: PeerId, context: C, channel: ResponseChannel<SharedSwapId>) {
        let shared_swap_id = SharedSwapId::default();

        tracing::info!("confirming swap as {}", shared_swap_id);

        self.inner.send_response(channel, shared_swap_id);
        self.events.push_back(BehaviourOutEvent::Confirmed {
            peer,
            shared_swap_id,
            context,
        })
    }

    fn abort(&mut self, peer: PeerId, context: C, channel: ResponseChannel<SharedSwapId>) {
        tracing::info!("aborting announce protocol with {}", peer);

        self.events
            .push_back(BehaviourOutEvent::Failed { peer, context });
        std::mem::drop(channel); // this closes the substream and reports an
                                 // error on the other end
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<RequestProtocol<AnnounceCodec>, BehaviourOutEvent<C>>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        Poll::Pending
    }
}

impl<C: fmt::Display + Send>
    NetworkBehaviourEventProcess<RequestResponseEvent<SwapDigest, SharedSwapId>> for Announce<C>
{
    fn inject_event(&mut self, event: RequestResponseEvent<SwapDigest, SharedSwapId>) {
        match event {
            RequestResponseEvent::Message {
                peer,
                message:
                    RequestResponseMessage::Request {
                        request, channel, ..
                    },
            } => {
                let span = tracing::info_span!("incoming_announcement", digest = %request);
                let _enter = span.enter();

                match self.awaiting_announcements.remove(&request) {
                    Some(awaiting_announcement) if awaiting_announcement.is_from(&peer) => {
                        self.confirm(peer, awaiting_announcement.context, channel)
                    }
                    Some(awaiting_announcement) => {
                        self.abort(peer, awaiting_announcement.context, channel);
                    }
                    None => {
                        tracing::info!("announcement hasn't been awaited yet, buffering it");
                        self.received_announcements
                            .insert(request, ReceivedAnnouncement {
                                from: peer,
                                channel,
                            });
                    }
                }
            }
            RequestResponseEvent::Message {
                peer,
                message:
                    RequestResponseMessage::Response {
                        response: shared_swap_id,
                        request_id,
                    },
            } => {
                let context = self
                    .sent_announcements
                    .remove(&request_id)
                    .expect("must contain request id");

                tracing::info!(
                    "swap {} confirmed with shared id {}",
                    context,
                    shared_swap_id
                );

                self.events.push_back(BehaviourOutEvent::Confirmed {
                    peer,
                    shared_swap_id,
                    context,
                })
            }
            RequestResponseEvent::OutboundFailure {
                peer,
                request_id,
                error,
            } => {
                let local_swap_id = self.sent_announcements.remove(&request_id).unwrap();

                self.events.push_back(BehaviourOutEvent::Failed {
                    peer,
                    context: local_swap_id,
                });

                tracing::warn!("outbound failure: {:?}", error);
            }
            RequestResponseEvent::InboundFailure { error, .. } => {
                tracing::warn!("inbound failure: {:?}", error);
            }
        }
    }
}

/// Event emitted  by the `Announce` behaviour.
#[derive(Debug)]
pub enum BehaviourOutEvent<C> {
    Confirmed {
        /// The peer we have successfully confirmed the swap with.
        peer: PeerId,
        /// The shared swap id that we can now use to refer to this confirmed
        /// swap.
        shared_swap_id: SharedSwapId,
        /// The announce context provided by the caller.
        context: C,
    },

    /// The announcement for a given swap failed for some reason.
    /// More details error reporting will be added later.
    /// Most likely the announcement failed because it timed out.
    Failed { peer: PeerId, context: C },
}

#[derive(Clone, Copy, Debug)]
pub struct AnnounceProtocol;

impl ProtocolName for AnnounceProtocol {
    fn protocol_name(&self) -> &[u8] {
        b"/comit/swap/announce/1.0.0"
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AnnounceCodec;

#[async_trait::async_trait]
impl RequestResponseCodec for AnnounceCodec {
    type Protocol = AnnounceProtocol;
    type Request = SwapDigest;
    type Response = SharedSwapId;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let message = upgrade::read_one(io, 1024)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut de = serde_json::Deserializer::from_slice(&message);
        let swap_digest = SwapDigest::deserialize(&mut de)?;

        Ok(swap_digest)
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let message = upgrade::read_one(io, 1024)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut de = serde_json::Deserializer::from_slice(&message);
        let swap_id = SharedSwapId::deserialize(&mut de)?;

        Ok(swap_id)
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let bytes = serde_json::to_vec(&req)?;
        upgrade::write_one(io, &bytes).await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        shared_swap_id: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let bytes = serde_json::to_vec(&shared_swap_id)?;
        upgrade::write_one(io, &bytes).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::test::{await_events_or_timeout, connect, new_swarm};
    use libp2p::Swarm;
    use std::{future::Future, time::Duration};

    #[tokio::test]
    async fn given_bob_awaits_an_announcements_when_alice_sends_one_then_swap_is_confirmed() {
        let (mut alice_swarm, _, alice_id) = new_swarm(|_, _| Announce::default());
        let (mut bob_swarm, _, bob_id) = new_swarm(|_, _| Announce::default());
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let swap_digest = SwapDigest::random();

        bob_swarm.await_announcement(swap_digest.clone(), alice_id, 1);
        alice_swarm.announce_swap(swap_digest, bob_id, 2);

        assert_both_confirmed(alice_swarm.next(), bob_swarm.next()).await;
    }

    #[tokio::test]
    async fn given_alice_announces_swap_when_bob_awaits_it_within_timeout_then_swap_is_confirmed() {
        let incoming_announcement_buffer_expiry = Duration::from_secs(5);

        let (mut alice_swarm, _, alice_id) = new_swarm(|_, _| Announce::default());
        let (mut bob_swarm, _, bob_id) =
            new_swarm(|_, _| Announce::new(incoming_announcement_buffer_expiry));
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let swap_digest = SwapDigest::random();

        alice_swarm.announce_swap(swap_digest.clone(), bob_id, 1);
        let bob_event = await_announcement_with_delay(
            alice_id,
            &mut bob_swarm,
            swap_digest,
            2,
            Duration::from_secs(3),
        );

        assert_both_confirmed(alice_swarm.next(), bob_event).await;
    }

    #[tokio::test]
    async fn given_alice_announces_swap_when_bob_is_too_slow_then_announcement_times_out() {
        let incoming_announcement_buffer_expiry = Duration::from_secs(2);

        let (mut alice_swarm, _, alice_id) =
            new_swarm(|_, _| Announce::new(incoming_announcement_buffer_expiry));
        let (mut bob_swarm, _, bob_id) =
            new_swarm(|_, _| Announce::new(incoming_announcement_buffer_expiry));
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let swap_digest = SwapDigest::random();

        alice_swarm.announce_swap(swap_digest.clone(), bob_id, 1);
        let bob_event = await_announcement_with_delay(
            alice_id,
            &mut bob_swarm,
            swap_digest,
            2,
            Duration::from_secs(4),
        );

        assert_both_failed(alice_swarm.next(), bob_event).await;
    }

    #[tokio::test]
    async fn given_bob_receives_announcement_with_wrong_peer_id_then_error() {
        let (mut alice_swarm, ..) = new_swarm(|_, _| Announce::default());
        let (mut bob_swarm, _, bob_id) = new_swarm(|_, _| Announce::default());
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let swap_digest = SwapDigest::random();
        let definitely_not_alice_id = PeerId::random();

        alice_swarm.announce_swap(swap_digest.clone(), bob_id, 1);
        bob_swarm.await_announcement(swap_digest.clone(), definitely_not_alice_id, 2);

        assert_both_failed(alice_swarm.next(), bob_swarm.next()).await;
    }

    #[tokio::test]
    async fn given_bob_receives_announcement_with_wrong_digest_then_error() {
        // Sending a wrong digest only fails because of the timeout. Default is more
        // than our test so we need to set it lower.
        let incoming_announcement_buffer_expiry = Duration::from_secs(2);

        let (mut alice_swarm, _, alice_id) =
            new_swarm(|_, _| Announce::new(incoming_announcement_buffer_expiry));
        let (mut bob_swarm, _, bob_id) = new_swarm(|_, _| Announce::default());
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let alice_swap_digest = SwapDigest::random();
        let bob_swap_digest = SwapDigest::random();

        alice_swarm.announce_swap(alice_swap_digest, bob_id, 1);
        bob_swarm.await_announcement(bob_swap_digest, alice_id, 2);

        // bob is still waiting for an announcement for a different swap, hence we only
        // assert alice
        assert!(
            matches!(alice_swarm.next().await, BehaviourOutEvent::Failed { .. }),
            "announcement should fail on alice's side"
        );
    }

    async fn await_announcement_with_delay<C: fmt::Display + fmt::Debug + Send>(
        alice_id: PeerId,
        bob_swarm: &mut Swarm<Announce<C>>,
        swap_digest: SwapDigest,
        context: C,
        delay: Duration,
    ) -> BehaviourOutEvent<C> {
        // poll Bob's swarm for some time. We don't expect any events though
        while let Ok(event) = tokio::time::timeout(delay, bob_swarm.next()).await {
            panic!("unexpected event emitted: {:?}", event)
        }

        bob_swarm.await_announcement(swap_digest, alice_id.clone(), context);
        bob_swarm.next().await
    }

    async fn assert_both_confirmed<C: fmt::Debug>(
        alice_event: impl Future<Output = BehaviourOutEvent<C>>,
        bob_event: impl Future<Output = BehaviourOutEvent<C>>,
    ) {
        match await_events_or_timeout(alice_event, bob_event).await {
            (
                BehaviourOutEvent::Confirmed {
                    shared_swap_id: alice_event_swap_id,
                    ..
                },
                BehaviourOutEvent::Confirmed {
                    shared_swap_id: bob_event_swap_id,
                    ..
                },
            ) => {
                assert_eq!(alice_event_swap_id, bob_event_swap_id);
            }
            (alice_event, bob_event) => panic!("expected both parties to confirm the swap but alice emitted {:?} and bob emitted {:?}", alice_event, bob_event),
        }
    }

    async fn assert_both_failed<C>(
        alice_event: impl Future<Output = BehaviourOutEvent<C>>,
        bob_event: impl Future<Output = BehaviourOutEvent<C>>,
    ) {
        let (alice_event, bob_event) = await_events_or_timeout(alice_event, bob_event).await;

        assert!(
            matches!(alice_event, BehaviourOutEvent::Failed { .. }),
            "announcement should fail on alice's side"
        );
        assert!(
            matches!(bob_event, BehaviourOutEvent::Failed { .. }),
            "announcement should time out on bob's side"
        );
    }
}
