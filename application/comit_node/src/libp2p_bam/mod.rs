mod handler;
mod protocol;

pub use self::{handler::*, protocol::*};

use crate::{
    bam_ext::{FromBamHeader, ToBamHeader},
    comit_client::SwapReject,
    swap_protocols::{
        self,
        asset::{Asset, AssetKind},
        rfc003::{self, bob::BobSpawner, state_store::StateStore, CreateLedgerEvents, Ledger},
        LedgerEventDependencies, LedgerKind, MetadataStore, SwapId, SwapProtocol,
    },
};
use bam::{
    json::{OutgoingRequest, Response, ValidatedIncomingRequest},
    Status,
};
use futures::{
    task::{self, Task},
    Async, Future,
};
use libp2p::{
    core::{
        protocols_handler::IntoProtocolsHandler,
        swarm::{ConnectedPoint, NetworkBehaviour, NetworkBehaviourAction, PollParameters},
        ProtocolsHandler,
    },
    Multiaddr, NetworkBehaviour, PeerId,
};
use std::{
    collections::{hash_map::Entry, vec_deque::VecDeque, HashMap, HashSet},
    marker::PhantomData,
};
use tokio::prelude::{AsyncRead, AsyncWrite};

#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct Behaviour<TSubstream, T, S> {
    pub bam: Bam<TSubstream>,
    pub mdns: libp2p::mdns::Mdns<TSubstream>,

    #[behaviour(ignore)]
    pub bob: swap_protocols::bob::ProtocolDependencies<T, S>,
    #[behaviour(ignore)]
    pub task_executor: tokio::runtime::TaskExecutor,
}

#[derive(Debug)]
pub struct Bam<TSubstream> {
    marker: PhantomData<TSubstream>,

    events: VecDeque<NetworkBehaviourAction<PendingOutgoingRequest, PendingIncomingRequest>>,
    known_request_headers: HashMap<String, HashSet<String>>,
    current_task: Option<Task>,
    addresses: HashMap<PeerId, Vec<Multiaddr>>,
}

impl<TSubstream, T: MetadataStore<SwapId>, S: StateStore>
    libp2p::core::swarm::NetworkBehaviourEventProcess<PendingIncomingRequest>
    for Behaviour<TSubstream, T, S>
{
    fn inject_event(&mut self, event: PendingIncomingRequest) {
        let PendingIncomingRequest { request, channel } = event;

        let response = generate_response(&self.bob, request);

        let future = response.and_then(|response| {
            channel.send(response).unwrap();

            Ok(())
        });

        self.task_executor.spawn(future);
    }
}

fn generate_response<B: BobSpawner>(
    bob_protocol_dependencies: &B,
    mut request: ValidatedIncomingRequest,
) -> Box<dyn Future<Item = Response, Error = ()> + Send> {
    match request.request_type() {
        "SWAP" => {
            let protocol: SwapProtocol = bam::header!(request
                .take_header("protocol")
                .map(SwapProtocol::from_bam_header));
            match protocol {
                SwapProtocol::Rfc003 => {
                    let swap_id = SwapId::default();

                    let alpha_ledger = bam::header!(request
                        .take_header("alpha_ledger")
                        .map(LedgerKind::from_bam_header));
                    let beta_ledger = bam::header!(request
                        .take_header("beta_ledger")
                        .map(LedgerKind::from_bam_header));
                    let alpha_asset = bam::header!(request
                        .take_header("alpha_asset")
                        .map(AssetKind::from_bam_header));
                    let beta_asset = bam::header!(request
                        .take_header("beta_asset")
                        .map(AssetKind::from_bam_header));

                    match (alpha_ledger, beta_ledger, alpha_asset, beta_asset) {
                        (
                            LedgerKind::Bitcoin(alpha_ledger),
                            LedgerKind::Ethereum(beta_ledger),
                            AssetKind::Bitcoin(alpha_asset),
                            AssetKind::Ether(beta_asset),
                        ) => handle_request(
                            bob_protocol_dependencies,
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            bam::body!(request.take_body_as()),
                        ),
                        (
                            LedgerKind::Ethereum(alpha_ledger),
                            LedgerKind::Bitcoin(beta_ledger),
                            AssetKind::Ether(alpha_asset),
                            AssetKind::Bitcoin(beta_asset),
                        ) => handle_request(
                            bob_protocol_dependencies,
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            bam::body!(request.take_body_as()),
                        ),
                        (
                            LedgerKind::Bitcoin(alpha_ledger),
                            LedgerKind::Ethereum(beta_ledger),
                            AssetKind::Bitcoin(alpha_asset),
                            AssetKind::Erc20(beta_asset),
                        ) => handle_request(
                            bob_protocol_dependencies,
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            bam::body!(request.take_body_as()),
                        ),
                        (
                            LedgerKind::Ethereum(alpha_ledger),
                            LedgerKind::Bitcoin(beta_ledger),
                            AssetKind::Erc20(alpha_asset),
                            AssetKind::Bitcoin(beta_asset),
                        ) => handle_request(
                            bob_protocol_dependencies,
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            bam::body!(request.take_body_as()),
                        ),
                        (alpha_ledger, beta_ledger, alpha_asset, beta_asset) => {
                            log::warn!(
                                "swapping {:?} to {:?} from {:?} to {:?} is currently not supported", alpha_asset, beta_asset, alpha_ledger, beta_ledger
                            );
                            Box::new(futures::future::ok(Response::new(Status::RE(21))))
                        }
                    }
                }
                SwapProtocol::Unknown(protocol) => {
                    log::warn!("the swap protocol {} is currently not supported", protocol);
                    Box::new(futures::future::ok(Response::new(Status::RE(21))))
                }
            }
        }
        unknown_request_type => unimplemented!(),
    }
}

fn handle_request<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset, B: BobSpawner>(
    bob_spawner: &B,
    swap_id: SwapId,
    alpha_ledger: AL,
    beta_ledger: BL,
    alpha_asset: AA,
    beta_asset: BA,
    body: rfc003::messages::RequestBody<AL, BL>,
) -> Box<dyn Future<Item = Response, Error = ()> + Send + 'static>
where
    LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
{
    match bob_spawner.spawn(
        swap_id,
        rfc003::messages::Request::<AL, BL, AA, BA> {
            alpha_asset,
            beta_asset,
            alpha_ledger,
            beta_ledger,
            alpha_ledger_refund_identity: body.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: body.beta_ledger_redeem_identity,
            alpha_expiry: body.alpha_expiry,
            beta_expiry: body.beta_expiry,
            secret_hash: body.secret_hash,
        },
    ) {
        Ok(response_future) => Box::new(response_future.then(move |result| {
            let response = match result {
                Ok(Ok(response)) => {
                    let body = rfc003::messages::AcceptResponseBody::<AL, BL> {
                        beta_ledger_refund_identity: response.beta_ledger_refund_identity,
                        alpha_ledger_redeem_identity: response.alpha_ledger_redeem_identity,
                    };
                    Response::new(Status::OK(20)).with_body(
                        serde_json::to_value(body)
                            .expect("body should always serialize into serde_json::Value"),
                    )
                }
                // FIXME: the called code should not be able to produce a "Rejected" here
                // Rejected is for cases were we can automatically determine that a given swap
                // request cannot be processes. As soon as we can dispatch the
                // request (and therefore these branches here are activated), the
                // only valid negative outcome should be "Declined"
                //
                // As long as Alice and Bob use the same state machine, this is not possible though.
                Ok(Err(SwapReject::Rejected)) => Response::new(Status::SE(21)),
                Ok(Err(SwapReject::Declined { reason: None })) => Response::new(Status::SE(20)),
                Ok(Err(SwapReject::Declined {
                    reason: Some(reason),
                })) => Response::new(Status::SE(20)).with_header(
                    "REASON",
                    reason
                        .to_bam_header()
                        .expect("reason header shouldn't fail to serialize"),
                ),
                Err(_) => {
                    log::warn!(
                        "Failed to receive from oneshot channel for swap {}",
                        swap_id
                    );
                    Response::new(Status::SE(0))
                }
            };

            Ok(response)
        })),
        Err(e) => {
            log::error!("Unable to spawn Bob: {:?}", e);
            Box::new(futures::future::ok(Response::new(Status::RE(0))))
        }
    }
}

impl<TSubstream, T, S> libp2p::core::swarm::NetworkBehaviourEventProcess<libp2p::mdns::MdnsEvent>
    for Behaviour<TSubstream, T, S>
{
    fn inject_event(&mut self, event: libp2p::mdns::MdnsEvent) {
        if let libp2p::mdns::MdnsEvent::Discovered(addresses) = event {
            for (peer, address) in addresses {
                log::debug!("discovered {:?} at {:?}", peer, address)
            }
        }
    }
}

impl<TSubstream> Bam<TSubstream> {
    pub fn new(known_request_headers: HashMap<String, HashSet<String>>) -> Self {
        Self {
            marker: PhantomData,
            events: VecDeque::new(),
            known_request_headers,
            current_task: None,
            addresses: HashMap::new(),
        }
    }
}

impl<TSubstream> Bam<TSubstream> {
    pub fn send_request(
        &mut self,
        peer_id: PeerId,
        request: OutgoingRequest,
    ) -> Box<dyn Future<Item = Response, Error = ()> + Send> {
        let (sender, receiver) = futures::oneshot();

        let request = PendingOutgoingRequest {
            request,
            channel: sender,
        };

        self.events.push_back(NetworkBehaviourAction::DialPeer {
            peer_id: peer_id.clone(),
        });
        self.events.push_back(NetworkBehaviourAction::SendEvent {
            peer_id,
            event: request,
        });

        if let Some(task) = &self.current_task {
            task.notify()
        }

        Box::new(receiver.map_err(|_| {
            log::warn!(
                "Sender of response future was unexpectedly dropped before response was received."
            )
        }))
    }
}

impl<TSubstream> NetworkBehaviour for Bam<TSubstream>
where
    TSubstream: AsyncRead + AsyncWrite,
{
    type ProtocolsHandler = BamHandler<TSubstream>;
    type OutEvent = PendingIncomingRequest;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        BamHandler::new(self.known_request_headers.clone())
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        log::debug!("returning addresses for {}", peer_id);

        self.addresses
            .get(peer_id)
            .map(|addresses| addresses.clone())
            .unwrap_or_else(Vec::new)
    }

    fn inject_connected(&mut self, peer_id: PeerId, endpoint: ConnectedPoint) {
        // TODO: test if we need this!

        log::debug!("connected to {} at {:?}", peer_id, endpoint);

        let address = match endpoint {
            ConnectedPoint::Dialer { address } => address,
            ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
        };

        match self.addresses.entry(peer_id) {
            Entry::Occupied(mut entry) => {
                let addresses = entry.get_mut();
                addresses.push(address)
            }
            Entry::Vacant(entry) => {
                let addresses = vec![address];
                entry.insert(addresses);
            }
        }
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId, endpoint: ConnectedPoint) {
        log::debug!("disconnected from {} at {:?}", peer_id, endpoint);
    }

    fn inject_node_event(
        &mut self,
        _: PeerId,
        event: <<Self::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::OutEvent,
    ) {
        log::debug!("incoming request: {:?}", event.request);

        self.events
            .push_back(NetworkBehaviourAction::GenerateEvent(event))
    }

    fn poll(
        &mut self,
        _params: &mut PollParameters<'_>,
) -> Async<NetworkBehaviourAction<<<Self::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::InEvent, Self::OutEvent>>{
        log::debug!("polling behaviour - {} pending events", self.events.len());

        match self.events.pop_front() {
            Some(event) => {
                log::debug!("Emitting {:?}", event);

                if let NetworkBehaviourAction::SendEvent { peer_id, .. } = &event {
                    if !self.addresses.contains_key(peer_id) {
                        log::info!(
                            "not yet connected to {}, cannot send message",
                            peer_id.clone()
                        );

                        self.events.push_back(event);

                        self.current_task = Some(task::current());
                        return Async::NotReady;
                    }
                }

                return Async::Ready(event);
            }
            None => {
                log::debug!("Currently no events, storing current task");

                self.current_task = Some(task::current());
                return Async::NotReady;
            }
        }
    }
}
