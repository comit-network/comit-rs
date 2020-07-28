use crate::{OrderId, SharedSwapId};
use futures::{prelude::*, AsyncWriteExt};
use libp2p::{
    core::upgrade,
    request_response::{
        handler::RequestProtocol, ProtocolName, ProtocolSupport, RequestResponse,
        RequestResponseCodec, RequestResponseConfig, RequestResponseEvent, RequestResponseMessage,
        ResponseChannel,
    },
    swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters},
    NetworkBehaviour, PeerId,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io,
    task::{Context, Poll},
    time::Duration,
};
use tracing::debug;

/// The time we wait for a take order request to be confirmed or denied.
const REQUEST_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, Copy)]
pub struct TakeOrderProtocol;

impl ProtocolName for TakeOrderProtocol {
    fn protocol_name(&self) -> &[u8] {
        b"/comit/orderbook/take/1.0.0"
    }
}

/// The take libp2p network behaviour, used to take an order.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "poll")]
#[allow(missing_debug_implementations)]
pub struct TakeOrder {
    inner: RequestResponse<TakeOrderCodec>,
    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,
}
impl TakeOrder {
    /// Take an order i.e., send a take order request.
    // We assume takers want to act in the role of Alice.
    pub fn take_order(&mut self, maker: &PeerId, id: OrderId) {
        self.inner.send_request(maker, id);
    }

    /// Confirm a take order request.
    // We assume makers act in the role of Bob.
    pub fn confirm(
        &mut self,
        order_id: OrderId,
        channel: ResponseChannel<Response>,
        taker: PeerId,
    ) {
        let shared_swap_id = SharedSwapId::default();
        tracing::debug!(
            "confirming take order request with swap id: {}",
            shared_swap_id
        );

        self.inner.send_response(channel, Response::Confirmation {
            order_id,
            shared_swap_id,
        });

        self.events
            .push_back(BehaviourOutEvent::TakeOrderConfirmation {
                peer_id: taker,
                order_id,
                shared_swap_id,
            });
    }

    /// Deny a take order request.
    pub fn deny(&mut self, taker: PeerId, order_id: OrderId, channel: ResponseChannel<Response>) {
        self.events.push_back(BehaviourOutEvent::Failed {
            peer_id: taker,
            order_id,
        });
        self.inner.send_response(channel, Response::Error);
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<RequestProtocol<TakeOrderCodec>, BehaviourOutEvent>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        Poll::Pending
    }
}

impl Default for TakeOrder {
    fn default() -> Self {
        let mut config = RequestResponseConfig::default();
        config.set_request_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS));
        let behaviour = RequestResponse::new(
            TakeOrderCodec::default(),
            vec![(TakeOrderProtocol, ProtocolSupport::Full)],
            config,
        );

        TakeOrder {
            inner: behaviour,
            events: VecDeque::new(),
        }
    }
}

impl NetworkBehaviourEventProcess<RequestResponseEvent<OrderId, Response>> for TakeOrder {
    fn inject_event(&mut self, event: RequestResponseEvent<OrderId, Response>) {
        match event {
            RequestResponseEvent::Message {
                peer: peer_id,
                message:
                    RequestResponseMessage::Request {
                        request: order_id,
                        channel: response_channel,
                    },
            } => self.events.push_back(BehaviourOutEvent::TakeOrderRequest {
                peer_id,
                response_channel,
                order_id,
            }),
            RequestResponseEvent::Message {
                peer: peer_id,
                message:
                    RequestResponseMessage::Response {
                        request_id: _,
                        response:
                            Response::Confirmation {
                                order_id,
                                shared_swap_id,
                            },
                    },
            } => {
                self.events
                    .push_back(BehaviourOutEvent::TakeOrderConfirmation {
                        peer_id,
                        order_id,
                        shared_swap_id,
                    });
            }
            RequestResponseEvent::Message {
                peer: peer_id,
                message:
                    RequestResponseMessage::Response {
                        request_id: _,
                        response: Response::Error,
                    },
            } => {
                // This should be unreachable because we close the channel on error.
                tracing::error!("received take order response error from peer: {}", peer_id);
            }
            RequestResponseEvent::OutboundFailure { error, .. } => {
                tracing::warn!("outbound failure: {:?}", error);
            }
            RequestResponseEvent::InboundFailure { error, .. } => {
                tracing::warn!("inbound failure: {:?}", error);
            }
        }
    }
}

/// Event emitted  by the `TakeOrder` behaviour.
#[derive(Debug)]
pub enum BehaviourOutEvent {
    /// Event emitted within Bob's node when a take order request is received.
    TakeOrderRequest {
        /// The peer from whom request originated.
        peer_id: PeerId,
        /// Channel to send a confirm/deny response on.
        response_channel: ResponseChannel<Response>,
        /// The ID of the order peer wants to take.
        order_id: OrderId,
    },
    /// Event emitted in both Alice and Bob's node when a take order is
    /// confirmed.
    TakeOrderConfirmation {
        /// The peer from whom request originated.
        peer_id: PeerId,
        /// The ID of the order taken.
        order_id: OrderId,
        /// Identifier for the swap, used by the COMIT communication protocols.
        shared_swap_id: SharedSwapId,
    },
    /// Event emitted in Bob's node when a take order fails, for Alice we just
    /// close the channel to signal the error.
    Failed {
        /// The peer from whom request originated.
        peer_id: PeerId,
        /// The ID of the order peer wanted to take.
        order_id: OrderId,
    },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TakeOrderCodec;

/// The different responses we can send back as part of an announcement.
///
/// For now, this only includes a generic error variant in addition to the
/// confirmation because we simply close the connection in case of an error.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Response {
    Confirmation {
        order_id: OrderId,
        shared_swap_id: SharedSwapId,
    },
    Error,
}

#[async_trait::async_trait]
impl RequestResponseCodec for TakeOrderCodec {
    type Protocol = TakeOrderProtocol;
    type Request = OrderId;
    type Response = Response;

    /// Reads a take order request from the given I/O stream.
    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let message = upgrade::read_one(io, 1024)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut de = serde_json::Deserializer::from_slice(&message);
        let order_id = OrderId::deserialize(&mut de)?;
        debug!("read request order id: {}", order_id);

        Ok(order_id)
    }

    /// Reads a response (to a take order request) from the given I/O stream.
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
        let res = Response::deserialize(&mut de)?;

        Ok(res)
    }

    /// Writes a take order request to the given I/O stream.
    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!("writing request order id: {}", req);
        let bytes = serde_json::to_vec(&req)?;
        upgrade::write_one(io, &bytes).await?;

        Ok(())
    }

    /// Writes a response (to a take order request) to the given I/O stream.
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
            Response::Confirmation { .. } => {
                let bytes = serde_json::to_vec(&res)?;
                upgrade::write_one(io, &bytes).await?;
            }
            Response::Error => {
                debug!("closing write response channel");
                // For now, errors just close the substream. We can
                // send actual error responses at a later point. A
                // denied take order request is defined as an error.
                let _ = io.close().await;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{
        orderbook::MakerId,
        test::{await_events_or_timeout, new_connected_swarm_pair},
    };
    use libp2p::Swarm;

    #[tokio::test]
    async fn take_order_request_confirmation() {
        let (mut alice, mut bob) = new_connected_swarm_pair(|_| TakeOrder::default()).await;
        let order = crate::order::meaningless_test_order(MakerId::from(bob.peer_id.clone()));

        alice.swarm.take_order(&order.maker.into(), order.id);

        // Trigger request/response messages.
        poll_no_event(&mut alice.swarm).await;
        let bob_event = tokio::time::timeout(Duration::from_secs(2), bob.swarm.next())
            .await
            .expect("failed to get TakeOrderRequest event");

        let (alice_peer_id, channel, order_id) = match bob_event {
            BehaviourOutEvent::TakeOrderRequest {
                peer_id,
                response_channel,
                order_id,
            } => (peer_id, response_channel, order_id),
            _ => panic!("unexepected bob event"),
        };
        bob.swarm.confirm(order_id, channel, alice_peer_id);

        let (alice_event, bob_event) =
            await_events_or_timeout(alice.swarm.next(), bob.swarm.next()).await;
        match (alice_event, bob_event) {
            (
                BehaviourOutEvent::TakeOrderConfirmation {
                    peer_id: alice_got_peer_id,
                    order_id: alice_got_order_id,
                    shared_swap_id: alice_got_swap_id,
                },
                BehaviourOutEvent::TakeOrderConfirmation {
                    peer_id: bob_got_peer_id,
                    order_id: bob_got_order_id,
                    shared_swap_id: bob_got_swap_id,
                },
            ) => {
                assert_eq!(alice_got_peer_id, bob.peer_id);
                assert_eq!(bob_got_peer_id, alice.peer_id);

                assert_eq!(alice_got_order_id, order.id);
                assert_eq!(bob_got_order_id, alice_got_order_id);

                assert_eq!(alice_got_swap_id, bob_got_swap_id);
            }
            _ => panic!("failed to get take order confirmation"),
        }
    }

    // Poll the swarm for some time, we don't expect any events though.
    async fn poll_no_event(swarm: &mut Swarm<TakeOrder>) {
        let delay = Duration::from_secs(2);

        while let Ok(event) = tokio::time::timeout(delay, swarm.next()).await {
            panic!("unexpected event emitted: {:?}", event)
        }
    }
}
