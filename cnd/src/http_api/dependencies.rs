use crate::{
    dependencies,
    network::{DialInformation, Network, RequestError, SendRequest},
    seed::{Seed, SwapSeed},
    swap_protocols::{
        asset::Asset,
        metadata_store::{self, MetadataStore},
        rfc003::{
            self,
            create_ledger_events::CreateLedgerEvents,
            messages::AcceptResponseBody,
            state_machine::SwapStates,
            state_store::{self, StateStore},
            ActorState, Ledger, Spawn,
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

/// A struct for capturing dependencies that are needed within the HTTP API
/// controllers.
///
/// This is a facade that implements all the required traits and forwards them
/// to another implementation. This allows us to keep the number of arguments to
/// HTTP API controllers small and still access all the functionality we need.
#[allow(missing_debug_implementations)]
pub struct Dependencies<S> {
    pub dependencies: Arc<dependencies::Dependencies>,
    pub swarm: Arc<S>, // S is the libp2p Swarm within a mutex.
}

impl<S> Clone for Dependencies<S> {
    fn clone(&self) -> Self {
        Self {
            dependencies: Arc::clone(&self.dependencies),
            swarm: Arc::clone(&self.swarm),
        }
    }
}

impl<S> MetadataStore for Dependencies<S>
where
    S: Send + Sync + 'static,
{
    fn get(&self, key: SwapId) -> Result<Option<Metadata>, metadata_store::Error> {
        self.dependencies.metadata_store.get(key)
    }

    fn insert(&self, metadata: Metadata) -> Result<(), metadata_store::Error> {
        self.dependencies.metadata_store.insert(metadata)
    }

    fn all(&self) -> Result<Vec<Metadata>, metadata_store::Error> {
        self.dependencies.metadata_store.all()
    }
}

impl<S> StateStore for Dependencies<S>
where
    S: Send + Sync + 'static,
{
    fn insert<A: ActorState>(&self, key: SwapId, value: A) {
        self.dependencies.state_store.insert(key, value)
    }

    fn get<A: ActorState>(&self, key: &SwapId) -> Result<Option<A>, state_store::Error> {
        self.dependencies.state_store.get(key)
    }

    fn update<A: ActorState>(&self, key: &SwapId, update: SwapStates<A::AL, A::BL, A::AA, A::BA>) {
        self.dependencies.state_store.update::<A>(key, update)
    }
}

impl<S: Network> Network for Dependencies<S>
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

impl<S: SendRequest> SendRequest for Dependencies<S>
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

impl<S> Spawn for Dependencies<S>
where
    S: Send + Sync + 'static,
{
    #[allow(clippy::type_complexity)]
    fn spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        swap_request: rfc003::Request<AL, BL, AA, BA>,
        accept: AcceptResponseBody<AL, BL>,
    ) -> mpsc::UnboundedReceiver<SwapStates<AL, BL, AA, BA>>
    where
        LedgerConnectors: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
        S: Send + Sync + 'static,
    {
        self.dependencies.spawn(swap_request, accept)
    }
}

impl<S> SwapSeed for Dependencies<S>
where
    S: Send + Sync + 'static,
{
    fn swap_seed(&self, id: SwapId) -> Seed {
        self.dependencies.swap_seed(id)
    }
}
