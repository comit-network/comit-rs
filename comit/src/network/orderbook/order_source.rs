use crate::{
    expiries::{AlphaOffset, BetaOffset},
    order::SwapProtocol,
    BtcDaiOrder, Price, Quantity,
};
use futures::{AsyncRead, AsyncWrite};
use libp2p::{
    core::{
        connection::{ConnectionId, ListenerId},
        upgrade, ConnectedPoint,
    },
    request_response::{
        handler::{RequestProtocol, RequestResponseHandlerEvent},
        ProtocolName, ProtocolSupport, RequestResponse, RequestResponseCodec,
        RequestResponseConfig, RequestResponseEvent, RequestResponseMessage, ResponseChannel,
    },
    swarm::{NetworkBehaviour, NetworkBehaviourAction, PollParameters},
    Multiaddr, PeerId,
};
use serde::Deserialize;
use std::{
    collections::{HashSet, VecDeque},
    error::Error,
    io,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use time::NumericalDuration;

/// Wait at least this long before re-getting orders from a maker.
const POLLING_INTERVAL: Duration = Duration::from_secs(5);

/// A [NetworkBehaviour] that acts as a source for orders.
///
/// Orders are pulled regularly from a given set of makers. Every connection
/// established will be tried as a potential order source.
#[allow(missing_debug_implementations)]
pub struct OrderSource {
    get_orders: RequestResponse<GetBtcDaiOrdersCodec>,
    /// Makers we will attempt to get updated orders from.
    active_makers: HashSet<PeerId>,
    last_polled_makers_at: Instant,
    actions:
        VecDeque<NetworkBehaviourAction<RequestProtocol<GetBtcDaiOrdersCodec>, BehaviourOutEvent>>,
}

impl OrderSource {
    /// Start getting orders from this peer.
    pub fn start_getting_orders_from(&mut self, maker: PeerId) {
        self.active_makers.insert(maker);
    }

    pub fn stop_getting_orders_from(&mut self, maker: &PeerId) {
        self.active_makers.remove(maker);
    }

    /// Respond to a get orders request.
    pub fn send_orders(&mut self, handle: ResponseHandle, orders: Vec<BtcDaiOrder>) {
        self.get_orders.send_response(handle.0, orders);
    }

    fn is_time_to_update_orders(&self) -> bool {
        Instant::now().duration_since(self.last_polled_makers_at) > POLLING_INTERVAL
    }
}

impl NetworkBehaviour for OrderSource {
    type ProtocolsHandler =
        <RequestResponse<GetBtcDaiOrdersCodec> as NetworkBehaviour>::ProtocolsHandler;
    type OutEvent = BehaviourOutEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        self.get_orders.new_handler()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.get_orders.addresses_of_peer(peer_id)
    }

    fn inject_connected(&mut self, peer_id: &PeerId) {
        self.get_orders.inject_connected(peer_id);
        self.active_makers.insert(peer_id.clone());

        tracing::debug!("connected to {}, attempting to get orders", peer_id);

        // try and get orders through this connection as soon as it is available
        self.get_orders.send_request(peer_id, ());
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        self.get_orders.inject_disconnected(peer_id);
        self.active_makers.remove(peer_id);
        self.actions
            .push_back(NetworkBehaviourAction::GenerateEvent(
                BehaviourOutEvent::MakerIsGone {
                    maker: peer_id.clone(),
                },
            ));
    }

    fn inject_connection_established(
        &mut self,
        peer: &PeerId,
        connection_id: &ConnectionId,
        connected_point: &ConnectedPoint,
    ) {
        self.get_orders
            .inject_connection_established(peer, connection_id, connected_point)
    }

    fn inject_connection_closed(
        &mut self,
        peer: &PeerId,
        connection_id: &ConnectionId,
        connected_point: &ConnectedPoint,
    ) {
        self.get_orders
            .inject_connection_closed(peer, connection_id, connected_point);
    }

    fn inject_address_change(
        &mut self,
        peer: &PeerId,
        connection_id: &ConnectionId,
        old: &ConnectedPoint,
        new: &ConnectedPoint,
    ) {
        self.get_orders
            .inject_address_change(peer, connection_id, old, new)
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: RequestResponseHandlerEvent<GetBtcDaiOrdersCodec>,
    ) {
        self.get_orders.inject_event(peer_id, connection, event)
    }

    fn inject_addr_reach_failure(
        &mut self,
        peer_id: Option<&PeerId>,
        addr: &Multiaddr,
        error: &dyn Error,
    ) {
        self.get_orders
            .inject_addr_reach_failure(peer_id, addr, error)
    }

    fn inject_dial_failure(&mut self, peer_id: &PeerId) {
        self.get_orders.inject_dial_failure(peer_id)
    }

    fn inject_new_listen_addr(&mut self, addr: &Multiaddr) {
        self.get_orders.inject_new_listen_addr(addr)
    }

    fn inject_expired_listen_addr(&mut self, addr: &Multiaddr) {
        self.get_orders.inject_expired_listen_addr(addr)
    }

    fn inject_new_external_addr(&mut self, addr: &Multiaddr) {
        self.get_orders.inject_new_external_addr(addr)
    }

    fn inject_listener_error(&mut self, id: ListenerId, err: &(dyn Error + 'static)) {
        self.get_orders.inject_listener_error(id, err)
    }

    fn inject_listener_closed(&mut self, id: ListenerId, reason: Result<(), &std::io::Error>) {
        self.get_orders.inject_listener_closed(id, reason)
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<RequestProtocol<GetBtcDaiOrdersCodec>, Self::OutEvent>> {
        match self.get_orders.poll(cx, params) {
            Poll::Ready(NetworkBehaviourAction::GenerateEvent(event)) => match event {
                RequestResponseEvent::Message {
                    peer: _,
                    message:
                        RequestResponseMessage::Request {
                            channel: response_channel,
                            ..
                        },
                } => {
                    return Poll::Ready(NetworkBehaviourAction::GenerateEvent(
                        BehaviourOutEvent::GetOrdersRequest {
                            response_handle: ResponseHandle(response_channel),
                        },
                    ))
                }
                RequestResponseEvent::Message {
                    peer: peer_id,
                    message:
                        RequestResponseMessage::Response {
                            response: orders, ..
                        },
                } => {
                    tracing::debug!("fetched {} orders from {}", orders.len(), peer_id);

                    return Poll::Ready(NetworkBehaviourAction::GenerateEvent(
                        BehaviourOutEvent::RetrievedOrders {
                            maker: peer_id,
                            orders,
                        },
                    ));
                }
                RequestResponseEvent::OutboundFailure { error, peer, .. } => {
                    self.active_makers.remove(&peer);

                    tracing::info!("removing {} as a potential order source because we failed to establish a connection to them: {:?}", peer, error);
                }
                RequestResponseEvent::InboundFailure { error, .. } => {
                    // TODO: stop fetching orders from this peer?
                    tracing::warn!("inbound failure: {:?}", error);
                }
            },
            Poll::Ready(NetworkBehaviourAction::DialPeer { peer_id, condition }) => {
                return Poll::Ready(NetworkBehaviourAction::DialPeer { peer_id, condition })
            }
            Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                peer_id,
                event,
                handler,
            }) => {
                return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    event,
                    handler,
                })
            }
            Poll::Ready(NetworkBehaviourAction::DialAddress { address }) => {
                return Poll::Ready(NetworkBehaviourAction::DialAddress { address })
            }
            Poll::Ready(NetworkBehaviourAction::ReportObservedAddr { address }) => {
                return Poll::Ready(NetworkBehaviourAction::ReportObservedAddr { address })
            }
            Poll::Pending => {}
        }

        if self.is_time_to_update_orders() {
            self.last_polled_makers_at = Instant::now();
            for id in &self.active_makers {
                self.get_orders.send_request(&id, ());
            }
        }

        if let Some(action) = self.actions.pop_front() {
            return Poll::Ready(action);
        }

        Poll::Pending
    }
}

impl Default for OrderSource {
    fn default() -> Self {
        let config = RequestResponseConfig::default();
        let behaviour = RequestResponse::new(
            GetBtcDaiOrdersCodec::default(),
            vec![(GetBtcDaiOrdersProtocol, ProtocolSupport::Full)],
            config,
        );

        Self {
            get_orders: behaviour,
            active_makers: HashSet::default(),
            last_polled_makers_at: Instant::now(),
            actions: VecDeque::default(),
        }
    }
}

#[derive(Debug)]
pub enum BehaviourOutEvent {
    /// Our orders are being requested by another peer.
    GetOrdersRequest { response_handle: ResponseHandle },
    /// We retrieved orders from the given maker.
    RetrievedOrders {
        maker: PeerId,
        orders: Vec<BtcDaiOrder>,
    },
    /// The given maker disconnected.
    ///
    /// It is unlikely that they will respond to any of their orders published
    /// in the past. We will also stop attempting to get new orders from
    /// them after this event has been emitted.
    MakerIsGone { maker: PeerId },
}

/// An opaque response handle required for sending back orders.
///
/// This type allows us to keep the `wire` module private to this module.
#[derive(Debug)]
pub struct ResponseHandle(ResponseChannel<Vec<BtcDaiOrder>>);

#[derive(Debug, Clone, Copy)]
pub struct GetBtcDaiOrdersProtocol;

impl ProtocolName for GetBtcDaiOrdersProtocol {
    fn protocol_name(&self) -> &[u8] {
        b"/comit/get-orders/btc-dai/1.0.0"
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GetBtcDaiOrdersCodec;

#[async_trait::async_trait]
impl RequestResponseCodec for GetBtcDaiOrdersCodec {
    type Protocol = GetBtcDaiOrdersProtocol;
    type Request = ();
    // TODO: Allow a response of "I am not a maker" to stop asking them.
    type Response = Vec<BtcDaiOrder>;

    /// Reads a get orders request from the given I/O stream.
    async fn read_request<T>(&mut self, _: &Self::Protocol, _: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        Ok(())
    }

    /// Reads a response (to a get orders request) from the given I/O stream.
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
        let orders = Vec::<wire::BtcDaiOrder>::deserialize(&mut de)?;

        Ok(orders.into_iter().map(|wire| wire.into_model()).collect())
    }

    /// Writes a get orders request to the given I/O stream.
    #[allow(clippy::unit_arg)]
    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        _: &mut T,
        _: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        Ok(())
    }

    /// Writes a response (to a get orders request) to the given I/O stream.
    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        orders: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let bytes = serde_json::to_vec(
            &orders
                .into_iter()
                .map(wire::BtcDaiOrder::from_model)
                .collect::<Vec<_>>(),
        )?;
        upgrade::write_one(io, &bytes).await?;

        Ok(())
    }
}

/// A dedicated module for the types that represent our messages "on the wire".
mod wire {
    use crate::{asset, asset::Erc20Quantity, OrderId, Position};
    use serde::{Deserialize, Serialize};
    use time::OffsetDateTime;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
    pub struct BtcDaiOrder {
        pub id: OrderId,
        pub position: Position,
        pub swap_protocol: SwapProtocol,
        #[serde(with = "time::serde::timestamp")]
        pub created_at: OffsetDateTime,
        #[serde(with = "asset::bitcoin::sats_as_string")]
        pub quantity: asset::Bitcoin,
        pub price: Erc20Quantity,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum SwapProtocol {
        HbitHerc20 {
            hbit_expiry_offset: i64,
            herc20_expiry_offset: i64,
        },
        Herc20Hbit {
            herc20_expiry_offset: i64,
            hbit_expiry_offset: i64,
        },
    }
}

impl wire::BtcDaiOrder {
    fn into_model(self) -> BtcDaiOrder {
        let wire::BtcDaiOrder {
            id,
            position,
            swap_protocol,
            created_at,
            quantity,
            price,
        } = self;

        BtcDaiOrder {
            id,
            position,
            swap_protocol: swap_protocol.into_model(),
            created_at,
            quantity: Quantity::new(quantity),
            price: Price::from_wei_per_sat(price),
        }
    }

    fn from_model(model: BtcDaiOrder) -> Self {
        let BtcDaiOrder {
            id,
            position,
            swap_protocol,
            created_at,
            quantity,
            price,
        } = model;

        Self {
            id,
            position,
            swap_protocol: wire::SwapProtocol::from_model(swap_protocol),
            created_at,
            quantity: quantity.to_inner(),
            price: price.wei_per_sat(), /* This is consistent with how we convert into the wire
                                         * model above. */
        }
    }
}

impl wire::SwapProtocol {
    fn into_model(self) -> SwapProtocol {
        match self {
            wire::SwapProtocol::Herc20Hbit {
                herc20_expiry_offset,
                hbit_expiry_offset,
            } => SwapProtocol::Herc20Hbit {
                herc20_expiry_offset: AlphaOffset::from(herc20_expiry_offset.seconds()),
                hbit_expiry_offset: BetaOffset::from(hbit_expiry_offset.seconds()),
            },
            wire::SwapProtocol::HbitHerc20 {
                hbit_expiry_offset,
                herc20_expiry_offset,
            } => SwapProtocol::HbitHerc20 {
                hbit_expiry_offset: AlphaOffset::from(hbit_expiry_offset.seconds()),
                herc20_expiry_offset: BetaOffset::from(herc20_expiry_offset.seconds()),
            },
        }
    }

    fn from_model(model: SwapProtocol) -> Self {
        use time::Duration;

        match model {
            SwapProtocol::HbitHerc20 {
                hbit_expiry_offset,
                herc20_expiry_offset,
            } => wire::SwapProtocol::HbitHerc20 {
                hbit_expiry_offset: Duration::from(hbit_expiry_offset).whole_seconds(),
                herc20_expiry_offset: Duration::from(herc20_expiry_offset).whole_seconds(),
            },
            SwapProtocol::Herc20Hbit {
                herc20_expiry_offset,
                hbit_expiry_offset,
            } => wire::SwapProtocol::Herc20Hbit {
                herc20_expiry_offset: Duration::from(herc20_expiry_offset).whole_seconds(),
                hbit_expiry_offset: Duration::from(hbit_expiry_offset).whole_seconds(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proptest;

    proptest::proptest! {
        #[test]
        fn conversions_to_and_from_wire_model_are_consistent(
            order in proptest::order::btc_dai_order(),
        ) {
            let round_tripped = wire::BtcDaiOrder::from_model(order.clone()).into_model();

            assert_eq!(order, round_tripped);
        }
    }
}
