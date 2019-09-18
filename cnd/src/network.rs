use crate::{
    libp2p_comit_ext::{FromHeader, ToHeader},
    swap_protocols::{
        asset::{Asset, AssetKind},
        rfc003::{
            self,
            bob::BobSpawner,
            messages::{Decision, DeclineResponseBody, SwapDeclineReason},
            CreateLedgerEvents,
        },
        HashFunction, LedgerEventDependencies, LedgerKind, SwapId, SwapProtocol,
    },
};
use futures::future::Future;
use libp2p::{
    core::muxing::{StreamMuxer, SubstreamRef},
    mdns::{Mdns, MdnsEvent},
    swarm::NetworkBehaviourEventProcess,
    Multiaddr, NetworkBehaviour, PeerId, Swarm, Transport,
};
use libp2p_comit::{
    frame::{OutboundRequest, Response, ValidatedInboundRequest},
    BehaviourOutEvent, Comit, PendingInboundRequest,
};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    fmt::Display,
    io,
    sync::{Arc, Mutex},
};
use tokio::runtime::TaskExecutor;

#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct ComitNode<TSubstream, B> {
    comit: Comit<TSubstream>,
    mdns: Mdns<TSubstream>,

    #[behaviour(ignore)]
    bob: B,
    #[behaviour(ignore)]
    task_executor: TaskExecutor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DialInformation {
    pub peer_id: PeerId,
    pub address_hint: Option<Multiaddr>,
}

impl Display for DialInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self.address_hint {
            None => write!(f, "{}", self.peer_id),
            Some(address_hint) => write!(f, "{}@{}", self.peer_id, address_hint),
        }
    }
}

impl<TSubstream, B> ComitNode<TSubstream, B> {
    pub fn new(bob: B, task_executor: TaskExecutor) -> Result<Self, io::Error> {
        let mut swap_headers = HashSet::new();
        swap_headers.insert("id".into());
        swap_headers.insert("alpha_ledger".into());
        swap_headers.insert("beta_ledger".into());
        swap_headers.insert("alpha_asset".into());
        swap_headers.insert("beta_asset".into());
        swap_headers.insert("protocol".into());

        let mut known_headers = HashMap::new();
        known_headers.insert("SWAP".into(), swap_headers);

        Ok(Self {
            comit: Comit::new(known_headers),
            mdns: Mdns::new()?,
            bob,
            task_executor,
        })
    }

    pub fn send_request(
        &mut self,
        peer_id: DialInformation,
        request: OutboundRequest,
    ) -> Box<dyn Future<Item = Response, Error = ()> + Send> {
        self.comit
            .send_request((peer_id.peer_id, peer_id.address_hint), request)
    }
}

pub trait SwarmInfo: Send + Sync + 'static {
    fn comit_peers(&self) -> Box<dyn Iterator<Item = (PeerId, Vec<Multiaddr>)> + Send + 'static>;
    fn listen_addresses(&self) -> Vec<Multiaddr>;
}

impl<
        TTransport: Transport + Send + 'static,
        B: BobSpawner + Send + 'static,
        TMuxer: StreamMuxer + Send + Sync + 'static,
    > SwarmInfo for Mutex<Swarm<TTransport, ComitNode<SubstreamRef<Arc<TMuxer>>, B>>>
where
    <TMuxer as StreamMuxer>::OutboundSubstream: Send + 'static,
    <TMuxer as StreamMuxer>::Substream: Send + 'static,
    <TTransport as Transport>::Dial: Send,
    <TTransport as Transport>::Error: Send,
    <TTransport as Transport>::Listener: Send,
    <TTransport as Transport>::ListenerUpgrade: Send,
    TTransport: Transport<Output = (PeerId, TMuxer)> + Clone,
{
    fn comit_peers(&self) -> Box<dyn Iterator<Item = (PeerId, Vec<Multiaddr>)> + Send + 'static> {
        let mut swarm = self.lock().unwrap();

        Box::new(swarm.comit.connected_peers())
    }

    fn listen_addresses(&self) -> Vec<Multiaddr> {
        let swarm = self.lock().unwrap();

        Swarm::listeners(&swarm)
            .chain(Swarm::external_addresses(&swarm))
            .cloned()
            .collect()
    }
}

impl<TSubstream, B: BobSpawner> NetworkBehaviourEventProcess<BehaviourOutEvent>
    for ComitNode<TSubstream, B>
{
    fn inject_event(&mut self, event: BehaviourOutEvent) {
        match event {
            BehaviourOutEvent::PendingInboundRequest { request, peer_id } => {
                let PendingInboundRequest { request, channel } = request;

                let generated_response = handle_request(&self.bob, peer_id, request);

                let future = generated_response
                    .and_then(|response| {
                        channel.send(response).unwrap_or_else(|_| {
                            log::debug!("failed to send response through channel")
                        });

                        Ok(())
                    })
                    .map_err(|_| unreachable!("error is Infallible"));

                self.task_executor.spawn(future);
            }
        }
    }
}

impl<TSubstream, B> NetworkBehaviourEventProcess<libp2p::mdns::MdnsEvent>
    for ComitNode<TSubstream, B>
{
    fn inject_event(&mut self, event: libp2p::mdns::MdnsEvent) {
        match event {
            MdnsEvent::Discovered(addresses) => {
                for (peer, address) in addresses {
                    log::trace!("discovered {} at {}", peer, address)
                }
            }
            MdnsEvent::Expired(addresses) => {
                for (peer, address) in addresses {
                    log::trace!("address {} of peer {} expired", address, peer)
                }
            }
        }
    }
}

fn handle_request<B: BobSpawner>(
    bob: &B,
    counterparty: PeerId,
    mut request: ValidatedInboundRequest,
) -> Box<dyn Future<Item = Response, Error = Infallible> + Send> {
    match request.request_type() {
        "SWAP" => {
            let protocol: SwapProtocol = header!(request
                .take_header("protocol")
                .map(SwapProtocol::from_header));
            match protocol {
                SwapProtocol::Rfc003(hash_function) => {
                    let swap_id = header!(request.take_header("id").map(SwapId::from_header));
                    let alpha_ledger = header!(request
                        .take_header("alpha_ledger")
                        .map(LedgerKind::from_header));
                    let beta_ledger = header!(request
                        .take_header("beta_ledger")
                        .map(LedgerKind::from_header));
                    let alpha_asset = header!(request
                        .take_header("alpha_asset")
                        .map(AssetKind::from_header));
                    let beta_asset = header!(request
                        .take_header("beta_asset")
                        .map(AssetKind::from_header));

                    match (alpha_ledger, beta_ledger, alpha_asset, beta_asset) {
                        (
                            LedgerKind::Bitcoin(alpha_ledger),
                            LedgerKind::Ethereum(beta_ledger),
                            AssetKind::Bitcoin(alpha_asset),
                            AssetKind::Ether(beta_asset),
                        ) => spawn(
                            bob,
                            counterparty,
                            rfc003_swap_request(
                                swap_id,
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                hash_function,
                                body!(request.take_body_as()),
                            ),
                        ),
                        (
                            LedgerKind::Ethereum(alpha_ledger),
                            LedgerKind::Bitcoin(beta_ledger),
                            AssetKind::Ether(alpha_asset),
                            AssetKind::Bitcoin(beta_asset),
                        ) => spawn(
                            bob,
                            counterparty,
                            rfc003_swap_request(
                                swap_id,
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                hash_function,
                                body!(request.take_body_as()),
                            ),
                        ),
                        (
                            LedgerKind::Bitcoin(alpha_ledger),
                            LedgerKind::Ethereum(beta_ledger),
                            AssetKind::Bitcoin(alpha_asset),
                            AssetKind::Erc20(beta_asset),
                        ) => spawn(
                            bob,
                            counterparty,
                            rfc003_swap_request(
                                swap_id,
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                hash_function,
                                body!(request.take_body_as()),
                            ),
                        ),
                        (
                            LedgerKind::Ethereum(alpha_ledger),
                            LedgerKind::Bitcoin(beta_ledger),
                            AssetKind::Erc20(alpha_asset),
                            AssetKind::Bitcoin(beta_asset),
                        ) => spawn(
                            bob,
                            counterparty,
                            rfc003_swap_request(
                                swap_id,
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                hash_function,
                                body!(request.take_body_as()),
                            ),
                        ),
                        (alpha_ledger, beta_ledger, alpha_asset, beta_asset) => {
                            log::warn!(
                                "swapping {:?} to {:?} from {:?} to {:?} is currently not supported", alpha_asset, beta_asset, alpha_ledger, beta_ledger
                            );

                            let decline_body = DeclineResponseBody {
                                reason: Some(SwapDeclineReason::UnsupportedSwap),
                            };

                            Box::new(futures::future::ok(
                                Response::empty()
                                    .with_header(
                                        "decision",
                                        Decision::Declined
                                            .to_header()
                                            .expect("Decision should not fail to serialize"),
                                    )
                                    .with_body(serde_json::to_value(decline_body).expect(
                                        "decline body should always serialize into serde_json::Value",
                                    )),
                            ))
                        }
                    }
                }
                SwapProtocol::Unknown(protocol) => {
                    log::warn!("the swap protocol {} is currently not supported", protocol);

                    let decline_body = DeclineResponseBody {
                        reason: Some(SwapDeclineReason::UnsupportedProtocol),
                    };
                    Box::new(futures::future::ok(
                        Response::empty()
                            .with_header(
                                "decision",
                                Decision::Declined
                                    .to_header()
                                    .expect("Decision should not fail to serialize"),
                            )
                            .with_body(serde_json::to_value(decline_body).expect(
                                "decline body should always serialize into serde_json::Value",
                            )),
                    ))
                }
            }
        }

        // This case is just catered for, because of rust. It can only happen
        // if there is a typo in the request_type within the program. The request
        // type is checked on the messaging layer and will be handled there if
        // an unknown request_type is passed in.
        request_type => {
            log::warn!("request type '{}' is unknown", request_type);

            Box::new(futures::future::ok(
                Response::empty().with_header(
                    "decision",
                    Decision::Declined
                        .to_header()
                        .expect("Decision should not fail to serialize"),
                ),
            ))
        }
    }
}

fn rfc003_swap_request<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset>(
    id: SwapId,
    alpha_ledger: AL,
    beta_ledger: BL,
    alpha_asset: AA,
    beta_asset: BA,
    hash_function: HashFunction,
    body: rfc003::messages::RequestBody<AL, BL>,
) -> rfc003::messages::Request<AL, BL, AA, BA> {
    rfc003::messages::Request::<AL, BL, AA, BA> {
        id,
        alpha_asset,
        beta_asset,
        alpha_ledger,
        beta_ledger,
        hash_function,
        alpha_ledger_refund_identity: body.alpha_ledger_refund_identity,
        beta_ledger_redeem_identity: body.beta_ledger_redeem_identity,
        alpha_expiry: body.alpha_expiry,
        beta_expiry: body.beta_expiry,
        secret_hash: body.secret_hash,
    }
}

fn spawn<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset, B: BobSpawner>(
    bob_spawner: &B,
    counterparty: PeerId,
    swap_request: rfc003::messages::Request<AL, BL, AA, BA>,
) -> Box<dyn Future<Item = Response, Error = Infallible> + Send + 'static>
where
    LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
{
    let swap_id = swap_request.id.clone();

    match bob_spawner.spawn(counterparty, swap_request) {
        Ok(response_future) => Box::new(response_future.then(move |result| {
            let response = match result {
                Ok(Ok(accept_body)) => {
                    let body = rfc003::messages::AcceptResponseBody::<AL, BL> {
                        beta_ledger_refund_identity: accept_body.beta_ledger_refund_identity,
                        alpha_ledger_redeem_identity: accept_body.alpha_ledger_redeem_identity,
                    };
                    Response::empty()
                        .with_header(
                            "decision",
                            Decision::Accepted
                                .to_header()
                                .expect("Decision should not fail to serialize"),
                        )
                        .with_body(
                            serde_json::to_value(body)
                                .expect("body should always serialize into serde_json::Value"),
                        )
                }
                Ok(Err(decline_body)) => Response::empty()
                    .with_header(
                        "decision",
                        Decision::Declined
                            .to_header()
                            .expect("Decision shouldn't fail to serialize"),
                    )
                    .with_body(
                        serde_json::to_value(decline_body)
                            .expect("decline body should always serialize into serde_json::Value"),
                    ),
                Err(_) => {
                    log::warn!(
                        "Failed to receive from oneshot channel for swap {}",
                        swap_id
                    );
                    Response::empty().with_header(
                        "decision",
                        Decision::Declined
                            .to_header()
                            .expect("Decision should not fail to serialize"),
                    )
                }
            };

            Ok(response)
        })),
        Err(e) => {
            log::error!("Unable to spawn Bob: {:?}", e);
            Box::new(futures::future::ok(
                Response::empty().with_header(
                    "decision",
                    Decision::Declined
                        .to_header()
                        .expect("Decision should not fail to serialize"),
                ),
            ))
        }
    }
}
