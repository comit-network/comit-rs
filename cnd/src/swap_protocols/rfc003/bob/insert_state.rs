use crate::swap_protocols::{
    asset::Asset,
    dependencies,
    metadata_store::{self, Metadata, MetadataStore, Role},
    rfc003::{self, bob, state_store::StateStore, Ledger},
};
use http_api_problem::HttpApiProblem;
use libp2p::PeerId;
use std::sync::Arc;

#[derive(Debug)]
pub enum Error {
    Metadata(metadata_store::Error),
}

impl From<Error> for HttpApiProblem {
    fn from(e: Error) -> Self {
        use self::Error::*;
        match e {
            Metadata(e) => e.into(),
        }
    }
}

pub trait InsertState: Send + Sync + 'static {
    #[allow(clippy::type_complexity)]
    fn insert_state<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        counterparty: PeerId,
        swap_request: rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> Result<(), Error>;
}

impl InsertState for dependencies::bob::ProtocolDependencies {
    #[allow(clippy::type_complexity)]
    fn insert_state<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        counterparty: PeerId,
        swap_request: rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> Result<(), Error> {
        let id = swap_request.id;
        let seed = self.seed.swap_seed(id);
        let bob = bob::State::proposed(swap_request.clone(), seed);

        let metadata = Metadata::new(
            id,
            swap_request.alpha_ledger.into(),
            swap_request.beta_ledger.into(),
            swap_request.alpha_asset.into(),
            swap_request.beta_asset.into(),
            Role::Bob,
            counterparty,
        );

        self.metadata_store
            .insert(metadata)
            .map_err(Error::Metadata)?;

        let state_store = Arc::clone(&self.state_store);
        state_store.insert(id, bob);

        Ok(())
    }
}
