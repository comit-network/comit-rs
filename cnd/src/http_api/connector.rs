use crate::{
    dependencies::Dependencies,
    network::{DialInformation, Network, RequestError, SendRequest},
    seed::{Seed, SwapSeed},
    swap_protocols::{
        asset::Asset,
        metadata_store::{self, MetadataStore},
        rfc003::{
            self,
            alice::SpawnAlice,
            bob::SpawnBob,
            create_ledger_events::CreateLedgerEvents,
            state_machine::SwapStates,
            state_store::{self, StateStore},
            ActorState, Ledger,
        },
        LedgerConnectors, Metadata, SwapId,
    },
};
use futures::{
    sync::{mpsc, oneshot::Sender},
    Future,
};
use libp2p::PeerId;
use libp2p_comit::frame::Response;
use std::sync::Arc;

/// Collect all the connector trait bounds together under one trait.
pub trait Connect:
    Clone + MetadataStore + StateStore + Network + SendRequest + SpawnAlice + SpawnBob + SwapSeed
{
}

#[allow(missing_debug_implementations)]
pub struct Connector<S> {
    pub deps: Arc<Dependencies>,
    pub swarm: Arc<S>, // S is the libp2p Swarm within a mutex.
}

impl<S> Connect for Connector<S> where S: Network + SendRequest {}

impl<S> Clone for Connector<S> {
    fn clone(&self) -> Self {
        Self {
            deps: Arc::clone(&self.deps),
            swarm: Arc::clone(&self.swarm),
        }
    }
}

impl<S> MetadataStore for Connector<S>
where
    S: Send + Sync + 'static,
{
    fn get(&self, key: SwapId) -> Result<Option<Metadata>, metadata_store::Error> {
        self.deps.metadata_store.get(key)
    }

    fn insert(&self, metadata: Metadata) -> Result<(), metadata_store::Error> {
        self.deps.metadata_store.insert(metadata)
    }

    fn all(&self) -> Result<Vec<Metadata>, metadata_store::Error> {
        self.deps.metadata_store.all()
    }
}

impl<S> StateStore for Connector<S>
where
    S: Send + Sync + 'static,
{
    fn insert<A: ActorState>(&self, key: SwapId, value: A) {
        self.deps.state_store.insert(key, value)
    }

    fn get<A: ActorState>(&self, key: &SwapId) -> Result<Option<A>, state_store::Error> {
        self.deps.state_store.get(key)
    }

    fn update<A: ActorState>(&self, key: &SwapId, update: SwapStates<A::AL, A::BL, A::AA, A::BA>) {
        self.deps.state_store.update::<A>(key, update)
    }
}

impl<S: Network> Network for Connector<S>
where
    S: Send + Sync + 'static,
{
    fn comit_peers(
        &self,
    ) -> Box<dyn Iterator<Item = (PeerId, Vec<libp2p::Multiaddr>)> + Send + 'static> {
        self.swarm.comit_peers()
    }

    fn listen_addresses(&self) -> Vec<libp2p::Multiaddr> {
        self.swarm.listen_addresses()
    }

    fn pending_request_for(&self, swap: SwapId) -> Option<Sender<Response>> {
        self.swarm.pending_request_for(swap)
    }
}

impl<S> SpawnBob for Connector<S>
where
    S: Send + Sync + 'static,
{
    #[allow(clippy::type_complexity)]
    fn spawn_bob<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        swap_request: rfc003::Request<AL, BL, AA, BA>,
        response: rfc003::Response<AL, BL>,
    ) -> mpsc::UnboundedReceiver<SwapStates<AL, BL, AA, BA>>
    where
        LedgerConnectors: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
        S: Send + Sync + 'static,
    {
        self.deps.spawn_bob(swap_request, response)
    }
}

impl<S> SpawnAlice for Connector<S>
where
    S: Send + Sync + 'static,
{
    #[allow(clippy::type_complexity)]
    fn spawn_alice<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        swap_request: rfc003::Request<AL, BL, AA, BA>,
        response: rfc003::Response<AL, BL>,
    ) -> mpsc::UnboundedReceiver<SwapStates<AL, BL, AA, BA>>
    where
        LedgerConnectors: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
        S: Send + Sync + 'static,
    {
        self.deps.spawn_alice(swap_request, response)
    }
}

impl<S: SendRequest> SendRequest for Connector<S>
where
    S: Send + Sync + 'static,
{
    fn send_request<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset>(
        &self,
        dial_info: DialInformation,
        request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Box<dyn Future<Item = rfc003::Response<AL, BL>, Error = RequestError> + Send>
    where
        LedgerConnectors: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
    {
        self.swarm.send_request(dial_info, request)
    }
}

impl<S> SwapSeed for Connector<S>
where
    S: Send + Sync + 'static,
{
    fn swap_seed(&self, id: SwapId) -> Seed {
        self.deps.swap_seed(id)
    }
}
