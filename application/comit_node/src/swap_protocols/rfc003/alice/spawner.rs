use crate::swap_protocols::{
    asset::Asset,
    dependencies::{LedgerEventDependencies, ProtocolDependencies},
    metadata_store::{self, Metadata, MetadataStore},
    rfc003::{
        alice::SwapRequest,
        state_store::{self, StateStore},
        Alice, CreateLedgerEvents, Initiation, Ledger, SecretSource,
    },
    SwapId,
};

use futures::Future;
use http_api_problem::HttpApiProblem;

#[derive(Debug)]
pub enum Error {
    Storage(state_store::Error),
    Metadata(metadata_store::Error),
}

impl From<Error> for HttpApiProblem {
    fn from(e: Error) -> Self {
        use self::Error::*;
        match e {
            Storage(e) => e.into(),
            Metadata(e) => e.into(),
        }
    }
}

pub trait AliceSpawner: Send + Sync + 'static {
    fn spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        id: SwapId,
        swap_request: SwapRequest<AL, BL, AA, BA>,
    ) -> Result<(), Error>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
        SwapRequest<AL, BL, AA, BA>: Into<Metadata>;
}

impl<T: MetadataStore<SwapId>, S: StateStore<SwapId>> AliceSpawner for ProtocolDependencies<T, S> {
    fn spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        id: SwapId,
        swap_request: SwapRequest<AL, BL, AA, BA>,
    ) -> Result<(), Error>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
        SwapRequest<AL, BL, AA, BA>: Into<Metadata>,
    {
        let save_state = self
            .state_store
            .new_save_state(id)
            .map_err(Error::Storage)?;
        self.metadata_store
            .insert(id, swap_request.clone())
            .map_err(Error::Metadata)?;

        let initiation = Initiation {
            alpha_asset: swap_request.alpha_asset,
            beta_asset: swap_request.beta_asset,
            alpha_ledger: swap_request.alpha_ledger,
            beta_ledger: swap_request.beta_ledger,
            beta_ledger_redeem_identity: swap_request.identities.beta_ledger_redeem_identity,
            alpha_ledger_refund_identity: swap_request.identities.alpha_ledger_refund_identity,
            alpha_expiry: swap_request.alpha_expiry,
            beta_expiry: swap_request.beta_expiry,
            secret: self.seed.new_secret(id),
        };

        let swap_execution = {
            Alice::<AL, BL, AA, BA>::new_state_machine(
                initiation,
                self.ledger_events.create_ledger_events(),
                self.ledger_events.create_ledger_events(),
                self.clone(),
                swap_request.bob_socket_address,
                save_state,
            )
        };

        tokio::spawn(
            swap_execution
                .map(move |outcome| {
                    info!("Swap {} finished with {:?}", id, outcome);
                })
                .map_err(move |e| {
                    error!("Swap {} failed with {:?}", id, e);
                }),
        );

        Ok(())
    }
}
