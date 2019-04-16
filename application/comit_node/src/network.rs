use crate::{
    comit_server::Server,
    libp2p_bam::{BamBehaviour, PendingIncomingRequest},
    swap_protocols::{bob, rfc003::state_store::StateStore, MetadataStore, SwapId},
};
use futures::future::Future;
use libp2p::{mdns::Mdns, NetworkBehaviour};
use tokio::runtime::TaskExecutor;

#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct Behaviour<TSubstream, T, S> {
    pub bam: BamBehaviour<TSubstream>,
    pub mdns: Mdns<TSubstream>,

    #[behaviour(ignore)]
    pub bob: bob::ProtocolDependencies<T, S>,
    #[behaviour(ignore)]
    pub task_executor: TaskExecutor,
}

impl<TSubstream, T: MetadataStore<SwapId>, S: StateStore>
    libp2p::core::swarm::NetworkBehaviourEventProcess<PendingIncomingRequest>
    for Behaviour<TSubstream, T, S>
{
    fn inject_event(&mut self, event: PendingIncomingRequest) {
        let PendingIncomingRequest { request, channel } = event;

        let response = self.bob.handle_request(request);

        let future = response.and_then(|response| {
            channel.send(response).unwrap();

            Ok(())
        });

        self.task_executor.spawn(future);
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
