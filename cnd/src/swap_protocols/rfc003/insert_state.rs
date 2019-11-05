use crate::{
    connector::Dependencies,
    swap_protocols::{
        asset::Asset,
        metadata_store::{self, Metadata, MetadataStore},
        rfc003::{self, alice, bob, state_store::StateStore, Ledger},
        Role,
    },
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
    fn insert_state_into_stores<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        role: Role,
        counterparty: PeerId,
        swap_request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Result<(), Error>;
}

impl InsertState for Dependencies {
    #[allow(clippy::type_complexity)]
    fn insert_state_into_stores<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        role: Role,
        counterparty: PeerId,
        swap_request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Result<(), Error> {
        let id = swap_request.id;
        let seed = self.seed.swap_seed(id);

        let metadata = Metadata::new(
            id,
            swap_request.alpha_ledger.into(),
            swap_request.beta_ledger.into(),
            swap_request.alpha_asset.into(),
            swap_request.beta_asset.into(),
            role,
            counterparty,
        );

        self.metadata_store
            .insert(metadata)
            .map_err(Error::Metadata)?;

        let state_store = Arc::clone(&self.state_store);
        match role {
            Role::Alice => {
                let state = alice::State::proposed(swap_request.clone(), seed);
                state_store.insert(id, state);
            }
            Role::Bob => {
                let state = bob::State::proposed(swap_request.clone(), seed);
                state_store.insert(id, state);
            }
        };

        Ok(())
    }
}
