use crate::{
    comit_client::Client,
    network::DialInformation,
    swap_protocols::{
        self,
        asset::Asset,
        dependencies::LedgerEventDependencies,
        metadata_store::{self, Metadata, MetadataStore, Role},
        rfc003::{
            alice::{self, State, SwapCommunication},
            messages::ToRequest,
            state_machine,
            state_store::{self, StateStore},
            CreateLedgerEvents, Ledger,
        },
        SwapId,
    },
};
use futures::{sync::mpsc, Stream};
use futures_core::{
    compat::Future01CompatExt,
    future::{FutureExt, TryFutureExt},
};
use http_api_problem::HttpApiProblem;
use std::sync::Arc;

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
        bob_dial_info: DialInformation,
        swap_request: Box<dyn ToRequest<AL, BL, AA, BA>>,
    ) -> Result<(), Error>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>;
}

impl<S: StateStore, C: Client> AliceSpawner for swap_protocols::alice::ProtocolDependencies<S, C> {
    fn spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        id: SwapId,
        bob_dial_info: DialInformation,
        partial_swap_request: Box<dyn ToRequest<AL, BL, AA, BA>>,
    ) -> Result<(), Error>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
    {
        let swap_seed = self.seed.swap_seed(id);
        let swap_request = partial_swap_request.to_request(id, &swap_seed);

        let metadata = Metadata::new(
            id,
            swap_request.alpha_ledger.into(),
            swap_request.beta_ledger.into(),
            swap_request.alpha_asset.into(),
            swap_request.beta_asset.into(),
            Role::Alice,
            bob_dial_info.peer_id.to_owned(),
        );

        self.metadata_store
            .insert(metadata)
            .map_err(Error::Metadata)?;

        let (sender, receiver) = mpsc::unbounded();

        let swap_execution = {
            let client = Arc::clone(&self.client);
            let ledger_events = self.ledger_events.clone();
            let state_store = Arc::clone(&self.state_store);

            async move {
                state_store.insert(id, State::proposed(swap_request.clone(), swap_seed));

                let alice_state = client
                    .send_rfc003_swap_request(bob_dial_info.clone(), swap_request.clone())
                    .compat()
                    .await
                    .map_err(|e| {
                        log::error!(
                            "Failed to send swap request to {} because {:?}",
                            bob_dial_info.peer_id,
                            e
                        );
                    })?
                    .map(|accept_response_body| {
                        State::accepted(swap_request.clone(), accept_response_body, swap_seed)
                    })
                    .unwrap_or_else(|decline_response_body| {
                        State::declined(swap_request.clone(), decline_response_body, swap_seed)
                    });

                state_store.insert(id, alice_state.clone());

                match alice_state {
                    State {
                        swap_communication: SwapCommunication::Accepted { request, response },
                        ..
                    } => {
                        let context = state_machine::Context {
                            alpha_ledger_events: ledger_events.create_ledger_events(),
                            beta_ledger_events: ledger_events.create_ledger_events(),
                            state_repo: Arc::new(sender),
                        };

                        let result = state_machine::Swap::start_in(
                            state_machine::Start {
                                swap: state_machine::OngoingSwap {
                                    alpha_ledger: request.alpha_ledger,
                                    beta_ledger: request.beta_ledger,
                                    alpha_asset: request.alpha_asset,
                                    beta_asset: request.beta_asset,
                                    hash_function: request.hash_function,
                                    alpha_ledger_redeem_identity: response
                                        .alpha_ledger_redeem_identity,
                                    alpha_ledger_refund_identity: request
                                        .alpha_ledger_refund_identity,
                                    beta_ledger_redeem_identity: request
                                        .beta_ledger_redeem_identity,
                                    beta_ledger_refund_identity: response
                                        .beta_ledger_refund_identity,
                                    alpha_expiry: request.alpha_expiry,
                                    beta_expiry: request.beta_expiry,
                                    secret_hash: request.secret_hash,
                                },
                            },
                            context,
                        )
                        .compat()
                        .await;

                        match result {
                            Ok(outcome) => log::info!("Swap {} finished with {:?}", id, outcome),
                            Err(e) => log::error!("Swap {} failed with {:?}", id, e),
                        }
                    }
                    _ => {
                        log::info!("Swap {} was declined", id);
                    }
                }

                Ok(())
            }
        };

        let state_store = Arc::clone(&self.state_store);
        tokio::spawn(receiver.for_each(move |update| {
            state_store.update::<alice::State<AL, BL, AA, BA>>(&id, update);
            Ok(())
        }));

        tokio::spawn(swap_execution.boxed().compat());

        Ok(())
    }
}
