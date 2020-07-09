use crate::{network::SwapDigest, LocalSwapId, SharedSwapId};
use futures::{prelude::*, AsyncWriteExt};
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
    io,
    task::{Context, Poll},
    time::Duration,
};

/// Implements the Announce (/comit/swap/announce/1.0.0) protocol.
///
/// We don't implement any connection handling here but assume that another
/// network behaviour handles connections and peer-ID to address translation for
/// us.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "poll")]
#[allow(missing_debug_implementations)]
pub struct Announce {
    inner: RequestResponse<AnnounceCodec>,

    #[behaviour(ignore)]
    awaiting_announcements: HashMap<SwapDigest, AwaitingAnnouncement>,
    #[behaviour(ignore)]
    sent_announcements: HashMap<RequestId, LocalSwapId>,
    #[behaviour(ignore)]
    received_announcements: HashMap<SwapDigest, ReceivedAnnouncement>,
    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,
}

#[derive(Debug)]
struct AwaitingAnnouncement {
    /// The peer we are awaiting the announcement from.
    from: PeerId,
    /// The swap ID we use locally to refer to this swap.
    local_swap_id: LocalSwapId,
}

impl AwaitingAnnouncement {
    fn is_from(&self, peer: &PeerId) -> bool {
        &self.from == peer
    }
}

#[derive(Debug)]
struct ReceivedAnnouncement {
    /// The peer we received the announcement from.
    from: PeerId,
    /// The channel we can use to send a response back.
    channel: ResponseChannel<Response>,
}

impl ReceivedAnnouncement {
    fn is_from(&self, peer: &PeerId) -> bool {
        &self.from == peer
    }
}

impl Default for Announce {
    fn default() -> Self {
        Self::new(Duration::from_secs(5 * 60))
    }
}

impl Announce {
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

    #[tracing::instrument(level = "info", skip(self))]
    pub fn announce_swap(&mut self, swap: SwapDigest, peer: PeerId, local_swap_id: LocalSwapId) {
        let request_id = self.inner.send_request(&peer, swap);

        tracing::info!("sending announcement with {:?}", request_id);

        self.sent_announcements.insert(request_id, local_swap_id);
    }

    #[tracing::instrument(level = "info", skip(self))]
    pub fn await_announcement(
        &mut self,
        swap: SwapDigest,
        peer: PeerId,
        local_swap_id: LocalSwapId,
    ) {
        match self.received_announcements.remove(&swap) {
            Some(received_announcement)
                if received_announcement.is_from(&peer)
                    && received_announcement.channel.is_open() =>
            {
                self.confirm(peer, local_swap_id, received_announcement.channel);
            }
            Some(received_announcement) => {
                self.abort(peer, local_swap_id, received_announcement.channel);
            }
            None => {
                tracing::info!("no pending announcement");

                self.awaiting_announcements
                    .insert(swap, AwaitingAnnouncement {
                        from: peer,
                        local_swap_id,
                    });
            }
        }
    }

    fn confirm(
        &mut self,
        peer: PeerId,
        local_swap_id: LocalSwapId,
        channel: ResponseChannel<Response>,
    ) {
        let shared_swap_id = SharedSwapId::default();

        tracing::info!("confirming swap as {}", shared_swap_id);

        self.inner
            .send_response(channel, Response::Confirmation(shared_swap_id));
        self.events.push_back(BehaviourOutEvent::Confirmed {
            peer,
            shared_swap_id,
            local_swap_id,
        })
    }

    fn abort(
        &mut self,
        peer: PeerId,
        local_swap_id: LocalSwapId,
        channel: ResponseChannel<Response>,
    ) {
        tracing::info!("aborting announce protocol with {}", peer);

        self.events.push_back(BehaviourOutEvent::Failed {
            peer,
            local_swap_id,
        });
        self.inner.send_response(channel, Response::Error);
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<RequestProtocol<AnnounceCodec>, BehaviourOutEvent>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        Poll::Pending
    }
}

impl NetworkBehaviourEventProcess<RequestResponseEvent<SwapDigest, Response>> for Announce {
    fn inject_event(&mut self, event: RequestResponseEvent<SwapDigest, Response>) {
        match event {
            RequestResponseEvent::Message {
                peer,
                message: RequestResponseMessage::Request { request, channel },
            } => {
                let span = tracing::info_span!("incoming_announcement", digest = %request);
                let _enter = span.enter();

                match self.awaiting_announcements.remove(&request) {
                    Some(awaiting_announcement) if awaiting_announcement.is_from(&peer) => {
                        self.confirm(peer, awaiting_announcement.local_swap_id, channel)
                    }
                    Some(awaiting_announcement) => {
                        self.abort(peer, awaiting_announcement.local_swap_id, channel);
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
                        response,
                        request_id,
                    },
            } => {
                // this is a bit awkward because we use the same code for Alice and Bob
                let shared_swap_id = match response {
                    Response::Confirmation(shared_swap_id) => shared_swap_id,
                    Response::Error => unimplemented!("we never emit this for Alice"),
                };

                let local_swap_id = self
                    .sent_announcements
                    .remove(&request_id)
                    .expect("must contain request id");

                tracing::info!(
                    "swap {} confirmed with shared id {}",
                    local_swap_id,
                    shared_swap_id
                );

                self.events.push_back(BehaviourOutEvent::Confirmed {
                    peer,
                    shared_swap_id,
                    local_swap_id,
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
                    local_swap_id,
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
pub enum BehaviourOutEvent {
    Confirmed {
        /// The peer we have successfully confirmed the swap with.
        peer: PeerId,
        /// The shared swap id that we can now use to refer to this confirmed
        /// swap.
        shared_swap_id: SharedSwapId,
        /// The swap id that we use locally to refer to this swap.
        local_swap_id: LocalSwapId,
    },

    /// The announcement for a given swap failed for some reason.
    /// More details error reporting will be added later.
    /// Most likely the announcement failed because it timed out.
    Failed {
        peer: PeerId,
        local_swap_id: LocalSwapId,
    },
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

/// The different responses we can send back as part of an announcement.
///
/// For now, this only includes a generic error variant in addition to the
/// confirmation because we simply close the connection in case of an error.
#[derive(Clone, Copy, Debug)]
pub enum Response {
    Confirmation(SharedSwapId),
    Error,
}

#[async_trait::async_trait]
impl RequestResponseCodec for AnnounceCodec {
    type Protocol = AnnounceProtocol;
    type Request = SwapDigest;
    type Response = Response;

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

        Ok(Response::Confirmation(swap_id))
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
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        match res {
            Response::Confirmation(shared_swap_id) => {
                let bytes = serde_json::to_vec(&shared_swap_id)?;
                upgrade::write_one(io, &bytes).await?;
            }
            Response::Error => {
                // for now, errors just close the substream.
                // we can send actual error responses at a later point
                let _ = io.close().await;
            }
        }

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
        let (mut alice_swarm, _, alice_id) = new_swarm(|_| Announce::default());
        let (mut bob_swarm, _, bob_id) = new_swarm(|_| Announce::default());
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let swap_digest = SwapDigest::random();

        bob_swarm.await_announcement(swap_digest.clone(), alice_id, LocalSwapId::default());
        alice_swarm.announce_swap(swap_digest, bob_id, LocalSwapId::default());

        assert_both_confirmed(alice_swarm.next(), bob_swarm.next()).await;
    }

    #[tokio::test]
    async fn given_alice_announces_swap_when_bob_awaits_it_within_timeout_then_swap_is_confirmed() {
        let incoming_announcement_buffer_expiry = Duration::from_secs(5);

        let (mut alice_swarm, _, alice_id) = new_swarm(|_| Announce::default());
        let (mut bob_swarm, _, bob_id) =
            new_swarm(|_| Announce::new(incoming_announcement_buffer_expiry));
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let swap_digest = SwapDigest::random();

        alice_swarm.announce_swap(swap_digest.clone(), bob_id, LocalSwapId::default());
        let bob_event = await_announcement_with_delay(
            alice_id,
            &mut bob_swarm,
            swap_digest,
            LocalSwapId::default(),
            Duration::from_secs(3),
        );

        assert_both_confirmed(alice_swarm.next(), bob_event).await;
    }

    #[tokio::test]
    async fn given_alice_announces_swap_when_bob_is_too_slow_then_announcement_times_out() {
        let incoming_announcement_buffer_expiry = Duration::from_secs(2);

        let (mut alice_swarm, _, alice_id) =
            new_swarm(|_| Announce::new(incoming_announcement_buffer_expiry));
        let (mut bob_swarm, _, bob_id) =
            new_swarm(|_| Announce::new(incoming_announcement_buffer_expiry));
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let swap_digest = SwapDigest::random();

        alice_swarm.announce_swap(swap_digest.clone(), bob_id, LocalSwapId::default());
        let bob_event = await_announcement_with_delay(
            alice_id,
            &mut bob_swarm,
            swap_digest,
            LocalSwapId::default(),
            Duration::from_secs(4),
        );

        assert_both_failed(alice_swarm.next(), bob_event).await;
    }

    #[tokio::test]
    async fn given_bob_receives_announcement_with_wrong_peer_id_then_error() {
        let (mut alice_swarm, ..) = new_swarm(|_| Announce::default());
        let (mut bob_swarm, _, bob_id) = new_swarm(|_| Announce::default());
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let swap_digest = SwapDigest::random();
        let definitely_not_alice_id = PeerId::random();

        alice_swarm.announce_swap(swap_digest.clone(), bob_id, LocalSwapId::default());
        bob_swarm.await_announcement(
            swap_digest.clone(),
            definitely_not_alice_id,
            LocalSwapId::default(),
        );

        assert_both_failed(alice_swarm.next(), bob_swarm.next()).await;
    }

    #[tokio::test]
    async fn given_bob_receives_announcement_with_wrong_digest_then_error() {
        // Sending a wrong digest only fails because of the timeout. Default is more
        // than our test so we need to set it lower.
        let incoming_announcement_buffer_expiry = Duration::from_secs(2);

        let (mut alice_swarm, _, alice_id) =
            new_swarm(|_| Announce::new(incoming_announcement_buffer_expiry));
        let (mut bob_swarm, _, bob_id) = new_swarm(|_| Announce::default());
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let alice_swap_digest = SwapDigest::random();
        let bob_swap_digest = SwapDigest::random();

        alice_swarm.announce_swap(alice_swap_digest, bob_id, LocalSwapId::default());
        bob_swarm.await_announcement(bob_swap_digest, alice_id, LocalSwapId::default());

        // bob is still waiting for an announcement for a different swap, hence we only
        // assert alice
        assert!(
            matches!(alice_swarm.next().await, BehaviourOutEvent::Failed { .. }),
            "announcement should fail on alice's side"
        );
    }

    async fn await_announcement_with_delay(
        alice_id: PeerId,
        bob_swarm: &mut Swarm<Announce>,
        swap_digest: SwapDigest,
        local_swap_id: LocalSwapId,
        delay: Duration,
    ) -> BehaviourOutEvent {
        // poll Bob's swarm for some time. We don't expect any events though
        while let Ok(event) = tokio::time::timeout(delay, bob_swarm.next()).await {
            panic!("unexpected event emitted: {:?}", event)
        }

        bob_swarm.await_announcement(swap_digest, alice_id.clone(), local_swap_id);
        bob_swarm.next().await
    }

    async fn assert_both_confirmed(
        alice_event: impl Future<Output = BehaviourOutEvent>,
        bob_event: impl Future<Output = BehaviourOutEvent>,
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

    async fn assert_both_failed(
        alice_event: impl Future<Output = BehaviourOutEvent>,
        bob_event: impl Future<Output = BehaviourOutEvent>,
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
