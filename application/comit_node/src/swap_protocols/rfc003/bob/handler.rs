use crate::{
    ledger_query_service::{DefaultLedgerQueryServiceApiClient, FirstMatch, QueryIdCache},
    seed::Seed,
    swap_protocols::{
        asset::Asset,
        metadata_store::MetadataStore,
        rfc003::{
            self,
            bob::SwapRequestKind,
            events::{BobToAlice, CommunicationEvents, LedgerEvents, LqsEvents, LqsEventsForErc20},
            roles::Bob,
            state_machine::*,
            state_store::StateStore,
            Ledger,
        },
        SwapId,
    },
};
use futures::{
    stream::Stream,
    sync::{mpsc::UnboundedReceiver, oneshot},
    Future,
};
use std::{sync::Arc, time::Duration};

#[derive(Debug)]
pub struct SwapRequestHandler<MetadataStore, StateStore> {
    // new dependencies
    pub receiver: UnboundedReceiver<(
        SwapId,
        SwapRequestKind,
        oneshot::Sender<rfc003::bob::SwapResponseKind>,
    )>,
    pub metadata_store: Arc<MetadataStore>,
    pub state_store: Arc<StateStore>,
    pub lqs_api_client: Arc<DefaultLedgerQueryServiceApiClient>,
    pub bitcoin_poll_interval: Duration,
    pub ethereum_poll_interval: Duration,
    pub seed: Seed,
}

impl<M: MetadataStore<SwapId>, S: StateStore<SwapId>> SwapRequestHandler<M, S> {
    pub fn start(self) -> impl Future<Item = (), Error = ()> {
        let (receiver, metadata_store, bitcoin_poll_interval, ethereum_poll_interval) = (
            self.receiver,
            self.metadata_store,
            self.bitcoin_poll_interval,
            self.ethereum_poll_interval,
        );
        let state_store = Arc::clone(&self.state_store);
        let lqs_api_client = Arc::clone(&self.lqs_api_client);

        receiver
            .for_each(move |(id, requests, response_sender)| {
                info!("Received swap {:?} on channel", id);
                match requests {
                    rfc003::bob::SwapRequestKind::BitcoinEthereumBitcoinQuantityEtherQuantity(
                        request,
                    ) => {
                        if let Err(e) = metadata_store.insert(id, request.clone()) {
                            error!("Failed to store metadata for swap {} because {:?}", id, e);

                            // Return Ok to keep the loop running
                            return Ok(());
                        }

                        {
                            let request = request.clone();
                            let (bob, response_future) = Bob::new();

                            let response_future = response_future.inspect(|response| {
                                response_sender
                                    .send(response.clone().into())
                                    .expect("receiver should never go out of scope");
                            });

                            let start_state = Start {
                                alpha_ledger_refund_identity: request.alpha_ledger_refund_identity,
                                beta_ledger_redeem_identity: request.beta_ledger_redeem_identity,
                                alpha_ledger: request.alpha_ledger,
                                beta_ledger: request.beta_ledger,
                                alpha_asset: request.alpha_asset,
                                beta_asset: request.beta_asset,
                                alpha_ledger_lock_duration: request.alpha_ledger_lock_duration,
                                secret: request.secret_hash,
                                role: bob,
                            };

                            spawn_state_machine(
                                id,
                                start_state,
                                state_store.as_ref(),
                                Box::new(LqsEvents::new(
                                    QueryIdCache::wrap(Arc::clone(&lqs_api_client)),
                                    FirstMatch::new(
                                        Arc::clone(&lqs_api_client),
                                        bitcoin_poll_interval,
                                    ),
                                )),
                                Box::new(LqsEvents::new(
                                    QueryIdCache::wrap(Arc::clone(&lqs_api_client)),
                                    FirstMatch::new(
                                        Arc::clone(&lqs_api_client),
                                        ethereum_poll_interval,
                                    ),
                                )),
                                Box::new(BobToAlice::new(Box::new(response_future))),
                            );
                        }

                        Ok(())
                    }
                    rfc003::bob::SwapRequestKind::BitcoinEthereumBitcoinQuantityErc20Quantity(
                        request,
                    ) => {
                        if let Err(e) = metadata_store.insert(id, request.clone()) {
                            error!("Failed to store metadata for swap {} because {:?}", id, e);

                            // Return Ok to keep the loop running
                            return Ok(());
                        }

                        {
                            let request = request.clone();
                            let (bob, response_future) = Bob::new();

                            let response_future = response_future.inspect(|response| {
                                response_sender
                                    .send(response.clone().into())
                                    .expect("receiver should never go out of scope");
                            });

                            let start_state = Start {
                                alpha_ledger_refund_identity: request.alpha_ledger_refund_identity,
                                beta_ledger_redeem_identity: request.beta_ledger_redeem_identity,
                                alpha_ledger: request.alpha_ledger,
                                beta_ledger: request.beta_ledger,
                                alpha_asset: request.alpha_asset,
                                beta_asset: request.beta_asset,
                                alpha_ledger_lock_duration: request.alpha_ledger_lock_duration,
                                secret: request.secret_hash,
                                role: bob,
                            };

                            spawn_state_machine(
                                id,
                                start_state,
                                state_store.as_ref(),
                                Box::new(LqsEvents::new(
                                    QueryIdCache::wrap(Arc::clone(&lqs_api_client)),
                                    FirstMatch::new(
                                        Arc::clone(&lqs_api_client),
                                        bitcoin_poll_interval,
                                    ),
                                )),
                                Box::new(LqsEventsForErc20::new(
                                    QueryIdCache::wrap(Arc::clone(&lqs_api_client)),
                                    FirstMatch::new(
                                        Arc::clone(&lqs_api_client),
                                        ethereum_poll_interval,
                                    ),
                                )),
                                Box::new(BobToAlice::new(Box::new(response_future))),
                            );
                        }

                        Ok(())
                    }
                }
            })
            .map_err(|_| ())
    }
}

fn spawn_state_machine<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset, S: StateStore<SwapId>>(
    id: SwapId,
    start_state: Start<Bob<AL, BL, AA, BA>>,
    state_store: &S,
    alpha_ledger_events: Box<LedgerEvents<AL, AA>>,
    beta_ledger_events: Box<LedgerEvents<BL, BA>>,
    communication_events: Box<CommunicationEvents<Bob<AL, BL, AA, BA>>>,
) {
    let state = SwapStates::Start(start_state);

    let save_state = state_store
        .insert(id, state.clone())
        .expect("handle errors :)"); // TODO: handle errors

    let context = Context {
        alpha_ledger_events,
        beta_ledger_events,
        state_repo: save_state,
        communication_events,
    };

    info!("Starting state machine for {:?}", id);
    tokio::spawn(
        Swap::start_in(state, context)
            .map(move |outcome| {
                info!("Swap {} finished with {:?}", id, outcome);
            })
            .map_err(move |e| {
                error!("Swap {} failed with {:?}", id, e);
            }),
    );
}
