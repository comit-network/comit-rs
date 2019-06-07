use crate::{
    bam_ext::{FromBamHeader, ToBamHeader},
    comit_client::SwapReject,
    libp2p_bam::{BamBehaviour, BehaviourOutEvent, PendingIncomingRequest},
    swap_protocols::{
        asset::{Asset, AssetKind},
        rfc003::{self, bob::BobSpawner, CreateLedgerEvents},
        LedgerEventDependencies, LedgerKind, SwapId, SwapProtocol,
    },
};
use bam::{
    self,
    json::{OutgoingRequest, Response, ValidatedIncomingRequest},
    Status,
};
use futures::future::Future;
use libp2p::{
    core::{
        muxing::{StreamMuxer, SubstreamRef},
        swarm::NetworkBehaviourEventProcess,
    },
    mdns::{Mdns, MdnsEvent},
    Multiaddr, NetworkBehaviour, PeerId, Swarm, Transport,
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
pub struct Behaviour<TSubstream, B> {
    bam: BamBehaviour<TSubstream>,
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

impl<TSubstream, B> Behaviour<TSubstream, B> {
    pub fn new(bob: B, task_executor: TaskExecutor) -> Result<Self, io::Error> {
        let mut swap_headers = HashSet::new();
        swap_headers.insert("alpha_ledger".into());
        swap_headers.insert("beta_ledger".into());
        swap_headers.insert("alpha_asset".into());
        swap_headers.insert("beta_asset".into());
        swap_headers.insert("protocol".into());

        let mut known_headers = HashMap::new();
        known_headers.insert("SWAP".into(), swap_headers);

        Ok(Self {
            bam: BamBehaviour::new(known_headers),
            mdns: Mdns::new()?,
            bob,
            task_executor,
        })
    }

    pub fn send_request(
        &mut self,
        peer_id: DialInformation,
        request: OutgoingRequest,
    ) -> Box<dyn Future<Item = Response, Error = ()> + Send> {
        self.bam.send_request(peer_id, request)
    }
}

pub trait SwarmInfo: Send + Sync + 'static {
    fn bam_peers(&self) -> Box<dyn Iterator<Item = (PeerId, Vec<Multiaddr>)> + Send + 'static>;
    fn listen_addresses(&self) -> Vec<Multiaddr>;
}

impl<
        TTransport: Transport + Send + 'static,
        B: BobSpawner + Send + 'static,
        TMuxer: StreamMuxer + Send + Sync + 'static,
    > SwarmInfo for Mutex<Swarm<TTransport, Behaviour<SubstreamRef<Arc<TMuxer>>, B>>>
where
    <TMuxer as StreamMuxer>::OutboundSubstream: Send + 'static,
    <TMuxer as StreamMuxer>::Substream: Send + 'static,
    <TTransport as Transport>::Dial: Send,
    <TTransport as Transport>::Error: Send,
    <TTransport as Transport>::Listener: Send,
    <TTransport as Transport>::ListenerUpgrade: Send,
    TTransport: Transport<Output = (PeerId, TMuxer)> + Clone,
{
    fn bam_peers(&self) -> Box<dyn Iterator<Item = (PeerId, Vec<Multiaddr>)> + Send + 'static> {
        let mut swarm = self.lock().unwrap();

        Box::new(swarm.bam.connected_peers())
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
    for Behaviour<TSubstream, B>
{
    fn inject_event(&mut self, event: BehaviourOutEvent) {
        match event {
            BehaviourOutEvent::PendingIncomingRequest { request, peer_id } => {
                let PendingIncomingRequest { request, channel } = request;

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
    for Behaviour<TSubstream, B>
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
    mut request: ValidatedIncomingRequest,
) -> Box<dyn Future<Item = Response, Error = Infallible> + Send> {
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
                        ) => spawn(
                            bob,
                            swap_id,
                            counterparty,
                            rfc003_swap_request(
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                bam::body!(request.take_body_as()),
                            ),
                        ),
                        (
                            LedgerKind::Ethereum(alpha_ledger),
                            LedgerKind::Bitcoin(beta_ledger),
                            AssetKind::Ether(alpha_asset),
                            AssetKind::Bitcoin(beta_asset),
                        ) => spawn(
                            bob,
                            swap_id,
                            counterparty,
                            rfc003_swap_request(
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                bam::body!(request.take_body_as()),
                            ),
                        ),
                        (
                            LedgerKind::Bitcoin(alpha_ledger),
                            LedgerKind::Ethereum(beta_ledger),
                            AssetKind::Bitcoin(alpha_asset),
                            AssetKind::Erc20(beta_asset),
                        ) => spawn(
                            bob,
                            swap_id,
                            counterparty,
                            rfc003_swap_request(
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                bam::body!(request.take_body_as()),
                            ),
                        ),
                        (
                            LedgerKind::Ethereum(alpha_ledger),
                            LedgerKind::Bitcoin(beta_ledger),
                            AssetKind::Erc20(alpha_asset),
                            AssetKind::Bitcoin(beta_asset),
                        ) => spawn(
                            bob,
                            swap_id,
                            counterparty,
                            rfc003_swap_request(
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                bam::body!(request.take_body_as()),
                            ),
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
        request_type => {
            log::warn!("request type '{}' is unknown", request_type);

            Box::new(futures::future::ok(Response::new(Status::SE(2))))
        }
    }
}

fn rfc003_swap_request<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset>(
    alpha_ledger: AL,
    beta_ledger: BL,
    alpha_asset: AA,
    beta_asset: BA,
    body: rfc003::messages::RequestBody<AL, BL>,
) -> rfc003::messages::Request<AL, BL, AA, BA> {
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
    }
}

fn spawn<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset, B: BobSpawner>(
    bob_spawner: &B,
    swap_id: SwapId,
    counterparty: PeerId,
    swap_request: rfc003::messages::Request<AL, BL, AA, BA>,
) -> Box<dyn Future<Item = Response, Error = Infallible> + Send + 'static>
where
    LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
{
    match bob_spawner.spawn(swap_id, counterparty, swap_request) {
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
