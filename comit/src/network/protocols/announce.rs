use crate::{network::SwapDigest, DialInformation, SharedSwapId};
use futures::prelude::*;
use libp2p::{
    core::upgrade,
    swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters},
    NetworkBehaviour, PeerId,
};
use libp2p_request_response::{
    handler::RequestProtocol, OutboundFailure, ProtocolName, ProtocolSupport, RequestId,
    RequestResponse, RequestResponseCodec, RequestResponseConfig, RequestResponseEvent,
    RequestResponseMessage, ResponseChannel,
};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    io,
    task::{Context, Poll},
    time::Duration,
};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "poll")]
pub struct Announce {
    inner: RequestResponse<AnnounceCodec>,

    #[behaviour(ignore)]
    pending_announcements: HashSet<SwapDigest>,
    #[behaviour(ignore)]
    sent_announcements: HashMap<RequestId, SwapDigest>,
    #[behaviour(ignore)]
    received_announcements: HashMap<SwapDigest, ResponseChannel<SharedSwapId>>,
    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,
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
            pending_announcements: Default::default(),
            sent_announcements: Default::default(),
            received_announcements: Default::default(),
            events: Default::default(),
        }
    }

    pub fn announce_swap(&mut self, swap: SwapDigest, dial_information: DialInformation) {
        if let Some(address) = dial_information.address_hint {
            self.inner.add_address(&dial_information.peer_id, address)
        }
        let request_id = self
            .inner
            .send_request(&dial_information.peer_id, swap.clone());
        self.sent_announcements.insert(request_id, swap);
    }

    pub fn await_announcement(&mut self, swap: SwapDigest, peer: PeerId) {
        match self.received_announcements.remove(&swap) {
            Some(channel) => {
                let shared_swap_id = SharedSwapId::default();

                self.inner.send_response(channel, shared_swap_id.clone());
                self.events.push_back(BehaviourOutEvent::Confirmed {
                    peer,
                    swap_digest: swap,
                    swap_id: shared_swap_id,
                });
            }
            None => {
                self.pending_announcements.insert(swap);
            }
        }
    }

    pub fn poll(
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

impl NetworkBehaviourEventProcess<RequestResponseEvent<SwapDigest, SharedSwapId>> for Announce {
    fn inject_event(&mut self, event: RequestResponseEvent<SwapDigest, SharedSwapId>) {
        match event {
            RequestResponseEvent::Message {
                peer,
                message: RequestResponseMessage::Request { request, channel },
            } => {
                if self.pending_announcements.contains(&request) {
                    let shared_swap_id = SharedSwapId::default();

                    self.inner.send_response(channel, shared_swap_id.clone());
                    self.events.push_back(BehaviourOutEvent::Confirmed {
                        peer,
                        swap_digest: request,
                        swap_id: shared_swap_id,
                    })
                } else {
                    self.received_announcements.insert(request, channel);
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
                let swap_digest = self
                    .sent_announcements
                    .remove(&request_id)
                    .expect("must contain request id");

                self.events.push_back(BehaviourOutEvent::Confirmed {
                    peer,
                    swap_digest,
                    swap_id: response,
                })
            }
            RequestResponseEvent::OutboundFailure {
                peer,
                request_id,
                error,
            } => match error {
                OutboundFailure::DialFailure => {}
                OutboundFailure::Timeout => self.events.push_back(BehaviourOutEvent::Timeout {
                    peer,
                    swap_digest: self.sent_announcements.remove(&request_id).unwrap(),
                }),
                OutboundFailure::ConnectionClosed => {}
                OutboundFailure::UnsupportedProtocols => {}
            },
            RequestResponseEvent::InboundFailure { peer, error } => {}
        }
    }
}

/// Event emitted  by the `Announce` behaviour.
#[derive(Debug)]
pub enum BehaviourOutEvent {
    Confirmed {
        /// The peer (Bob) that the swap has been announced to.
        peer: PeerId,
        /// The swap id returned by the peer (Bob).
        swap_id: SharedSwapId,
        /// The swap_digest
        swap_digest: SwapDigest,
    },

    /// Error while attempting to announce swap to the remote.
    Error {
        /// The peer with whom the error originated.
        peer: PeerId,
    },

    /// The announcement for a given swap timed out.
    Timeout {
        peer: PeerId,
        swap_digest: SwapDigest,
    },
}

#[derive(Clone)]
pub struct AnnounceProtocol;

impl ProtocolName for AnnounceProtocol {
    fn protocol_name(&self) -> &[u8] {
        b"/comit/swap/announce/1.0.0"
    }
}

#[derive(Clone, Default)]
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
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let bytes = serde_json::to_vec(&res)?;
        upgrade::write_one(io, &bytes).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{network::test_swarm, DialInformation};
    use futures::future;
    use libp2p::Swarm;
    use std::{future::Future, time::Duration};

    #[tokio::test]
    async fn given_bob_awaits_an_announcements_when_alice_sends_one_then_swap_is_confirmed() {
        let (mut alice_swarm, _, alice_id) = test_swarm::new(Announce::default());
        let (mut bob_swarm, bob_addr, bob_id) = test_swarm::new(Announce::default());
        let swap_digest = SwapDigest::random();

        bob_swarm.await_announcement(swap_digest.clone(), alice_id.clone());
        alice_swarm.announce_swap(swap_digest, DialInformation {
            peer_id: bob_id.clone(),
            address_hint: Some(bob_addr),
        });

        assert_both_confirmed(alice_swarm.next(), bob_swarm.next()).await;
    }

    #[tokio::test]
    async fn given_alice_announces_swap_when_bob_awaits_it_within_timeout_then_swap_is_confirmed() {
        let incoming_announcement_buffer_expiry = Duration::from_secs(5);

        let (mut alice_swarm, _, alice_id) = test_swarm::new(Announce::default());
        let (mut bob_swarm, bob_addr, bob_id) =
            test_swarm::new(Announce::new(incoming_announcement_buffer_expiry));
        let swap_digest = SwapDigest::random();

        alice_swarm.announce_swap(swap_digest.clone(), DialInformation {
            peer_id: bob_id.clone(),
            address_hint: Some(bob_addr),
        });
        let bob_event = await_announcement_with_delay(
            alice_id,
            &mut bob_swarm,
            swap_digest,
            Duration::from_secs(1),
        );

        assert_both_confirmed(alice_swarm.next(), bob_event).await;
    }

    #[tokio::test]
    async fn given_alice_announces_swap_when_bob_is_too_slow_then_announcement_times_out() {
        let incoming_announcement_buffer_expiry = Duration::from_secs(2);

        let (mut alice_swarm, _, alice_id) = test_swarm::new(Announce::default());
        let (mut bob_swarm, bob_addr, bob_id) =
            test_swarm::new(Announce::new(incoming_announcement_buffer_expiry));
        let swap_digest = SwapDigest::random();

        alice_swarm.announce_swap(swap_digest.clone(), DialInformation {
            peer_id: bob_id.clone(),
            address_hint: Some(bob_addr),
        });
        let bob_event = await_announcement_with_delay(
            alice_id,
            &mut bob_swarm,
            swap_digest,
            Duration::from_secs(4),
        );

        let (alice_event, bob_event) = await_events_or_timeout(alice_swarm.next(), bob_event).await;
        assert!(
            matches!(alice_event, BehaviourOutEvent::Error { .. }),
            "announcement should fail on alice's side"
        );
        assert!(
            matches!(bob_event, BehaviourOutEvent::Timeout { .. }),
            "announcement should time out on bob's side"
        );
    }

    async fn await_announcement_with_delay(
        alice_id: PeerId,
        bob_swarm: &mut Swarm<Announce>,
        swap_digest: SwapDigest,
        delay: Duration,
    ) -> BehaviourOutEvent {
        // poll Bob's swarm for some time. We don't expect any events though
        while let Ok(event) = tokio::time::timeout(delay, bob_swarm.next()).await {
            panic!("unexpected event emitted: {:?}", event)
        }

        bob_swarm.await_announcement(swap_digest, alice_id.clone());
        bob_swarm.next().await
    }

    async fn assert_both_confirmed(
        alice_event: impl Future<Output = BehaviourOutEvent>,
        bob_event: impl Future<Output = BehaviourOutEvent>,
    ) {
        match await_events_or_timeout(alice_event, bob_event).await {
            (
                BehaviourOutEvent::Confirmed {
                    swap_id: alice_event_swap_id,
                    swap_digest: alice_event_swap_digest,
                    ..
                },
                BehaviourOutEvent::Confirmed {
                    swap_id: bob_event_swap_id,
                    swap_digest: bob_event_swap_digest,
                    ..
                },
            ) => {
                assert_eq!(alice_event_swap_id, bob_event_swap_id);
                assert_eq!(alice_event_swap_digest, bob_event_swap_digest);
            }
            _ => panic!("expected both parties to confirm the swap"),
        }
    }

    async fn await_events_or_timeout(
        alice_event: impl Future<Output = BehaviourOutEvent>,
        bob_event: impl Future<Output = BehaviourOutEvent>,
    ) -> (BehaviourOutEvent, BehaviourOutEvent) {
        tokio::time::timeout(
            Duration::from_secs(10),
            future::join(alice_event, bob_event),
        )
        .await
        .expect("network behaviours to emit an event within 10 seconds")
    }
}
