use crate::{
    asset,
    http_api::routes::index::Body,
    identity,
    network::{comit_ln, DialInformation, Swarm},
    swap_protocols::{halight::InvoiceStates, LedgerStates, NodeLocalSwapId, Role, SwapId},
    timestamp::Timestamp,
};
use std::sync::Arc;

/// This represent the information available on a swap
/// before communication with the other node has started
/// TODO: Find a better place
/// TODO: Either make specific to han-halight or make it generic
#[derive(Clone, Debug)]
pub struct CreateSwapParams {
    pub role: Role,
    pub peer: DialInformation,
    pub ethereum_identity: identity::Ethereum,
    pub ethereum_absolute_expiry: Timestamp,
    pub ethereum_amount: asset::Ether,
    pub lightning_identity: identity::Lightning,
    pub lightning_cltv_expiry: Timestamp,
    pub lightning_amount: asset::Lightning,
}

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
        // TODO:  delegate to database
    }

    pub async fn initiate_communication(&self, id: NodeLocalSwapId, swap_params: CreateSwapParams) {
        self.swarm.initiate_communication(id, swap_params).await;
    }

    // TODO this should NodeLocalSwapId
    pub async fn get_finalized_swap(&self, id: SwapId) -> Option<comit_ln::FinalizedSwap> {
        // TODO this should read from the DB and not from the swarm
        self.swarm.get_finalized_swap(id).await
    }
}
