use crate::{
    comit_client,
    swap_protocols::{
        asset::Asset,
        dependencies::{LedgerEventDependencies, ProtocolDependencies},
        metadata_store::{self, Metadata, MetadataStore},
        rfc003::{
            bob::SwapRequest,
            create_ledger_events::CreateLedgerEvents,
            events::ResponseFuture,
            state_store::{self, StateStore},
            Bob, Initiation, Ledger,
        },
        SwapId,
    },
};

use futures::Future;
use http_api_problem::HttpApiProblem;

#[derive(Debug)]
pub enum SpawnError {
    Storage(state_store::Error),
    Metadata(metadata_store::Error),
}

impl From<SpawnError> for HttpApiProblem {
    fn from(e: SpawnError) -> Self {
        use self::SpawnError::*;
        match e {
            Storage(e) => e.into(),
            Metadata(e) => e.into(),
        }
    }
}

pub trait BobSpawner: Send + Sync + 'static {
    #[allow(clippy::type_complexity)]
    fn spawn_bob<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        id: SwapId,
        swap_request: SwapRequest<AL, BL, AA, BA>,
    ) -> Result<Box<ResponseFuture<Bob<AL, BL, AA, BA>>>, SpawnError>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
        SwapRequest<AL, BL, AA, BA>: Into<Metadata>;
}

impl<T: MetadataStore<SwapId>, S: StateStore<SwapId>, C: comit_client::Client> BobSpawner
    for ProtocolDependencies<T, S, C>
{
    #[allow(clippy::type_complexity)]
    fn spawn_bob<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        id: SwapId,
        swap_request: SwapRequest<AL, BL, AA, BA>,
    ) -> Result<Box<ResponseFuture<Bob<AL, BL, AA, BA>>>, SpawnError>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
        SwapRequest<AL, BL, AA, BA>: Into<Metadata>,
    {
        let save_state = self
            .state_store
            .new_save_state(id)
            .map_err(SpawnError::Storage)?;
        self.metadata_store
            .insert(id, swap_request.clone())
            .map_err(SpawnError::Metadata)?;

        let initiation = Initiation {
            alpha_asset: swap_request.alpha_asset,
            beta_asset: swap_request.beta_asset,
            alpha_ledger: swap_request.alpha_ledger,
            beta_ledger: swap_request.beta_ledger,
            beta_ledger_redeem_identity: swap_request.beta_ledger_redeem_identity,
            alpha_ledger_refund_identity: swap_request.alpha_ledger_refund_identity,
            alpha_ledger_lock_duration: swap_request.alpha_ledger_lock_duration,
            secret: swap_request.secret_hash,
        };

        let (state_machine_future, response_future) = Bob::new_state_machine(
            initiation,
            self.ledger_events.create_ledger_events(),
            self.ledger_events.create_ledger_events(),
            save_state,
        );

        tokio::spawn(
            state_machine_future
                .map(move |outcome| {
                    info!("Swap {} finished with {:?}", id, outcome);
                })
                .map_err(move |e| {
                    error!("Swap {} failed with {:?}", id, e);
                }),
        );

        Ok(response_future)
    }
}
