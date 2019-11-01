use crate::{
    comit_client::Client,
    swap_protocols::{
        asset::Asset,
        dependencies::alice::ProtocolDependencies,
        metadata_store::{Metadata, MetadataStore, Role},
        rfc003::{self, alice, insert_state::Error, state_store::StateStore, InsertState, Ledger},
    },
};
use libp2p::PeerId;
use std::sync::Arc;

impl<C: Client> InsertState for ProtocolDependencies<C> {
    #[allow(clippy::type_complexity)]
    fn insert_state_into_stores<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        counterparty: PeerId,
        swap_request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Result<(), Error> {
        let id = swap_request.id;
        let seed = self.seed.swap_seed(id);
        let state = alice::State::proposed(swap_request.clone(), seed);

        let metadata = Metadata::new(
            id,
            swap_request.alpha_ledger.into(),
            swap_request.beta_ledger.into(),
            swap_request.alpha_asset.into(),
            swap_request.beta_asset.into(),
            Role::Alice,
            counterparty,
        );

        self.metadata_store
            .insert(metadata)
            .map_err(Error::Metadata)?;

        let state_store = Arc::clone(&self.state_store);
        state_store.insert(id, state);

        Ok(())
    }
}
