use crate::{
    http_api::routes::index::Body,
    network::Swarm,
    swap_protocols::{halight::InvoiceStates, LedgerStates, NodeLocalSwapId, Role, SwapId},
};
use std::sync::Arc;

/// This is a facade that implements all the required traits and forwards them
/// to another implementation. This allows us to keep the number of arguments to
/// HTTP API controllers small and still access all the functionality we need.
#[derive(Clone, Debug)]
pub struct Facade2 {
    pub swarm: Swarm,
    pub alpha_ledger_state: Arc<LedgerStates>, // FIXME: For now this is Ethereum.
    pub beta_ledger_state: Arc<InvoiceStates>, // FIXME: For now this is HALight.
}

impl Facade2 {
    pub async fn save(&self, _id: NodeLocalSwapId, _body: Body) {
        // delegate to database

        unimplemented!()
    }

    pub async fn initiate_communication(&self, id: NodeLocalSwapId, body: Body) {
        self.swarm.initiate_communication(id, body).await;
    }

    // TODO this should NodeLocalSwapId
    pub async fn role_for_swap(&self, id: SwapId) -> Option<Role> {
        // TODO this should read from the DB and not from the swarm
        self.swarm.role_for_swap(id).await
    }
}
