use crate::{
    comit_client::Client,
    network::DialInformation,
    swap_protocols::{
        self,
        asset::Asset,
        dependencies::LedgerEventDependencies,
        rfc003::{
            self,
            alice::{spawner::AliceSpawner, State},
            create_ledger_events::CreateLedgerEvents,
            messages::ToRequest,
            state_store::StateStore,
            InsertState, Ledger,
        },
        SwapId,
    },
};
use futures_core::{
    compat::Future01CompatExt,
    future::{FutureExt, TryFutureExt},
};
use std::sync::Arc;

pub trait InitiateSwapRequest: Send + Sync + 'static {
    fn initiate_swap_request<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        id: SwapId,
        bob_dial_info: DialInformation,
        partial_swap_request: Box<dyn ToRequest<AL, BL, AA, BA>>,
    ) -> Result<(), rfc003::insert_state::Error>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>;
}

impl<S: Client> InitiateSwapRequest for swap_protocols::alice::ProtocolDependencies<S> {
    fn initiate_swap_request<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        id: SwapId,
        bob_dial_info: DialInformation,
        partial_swap_request: Box<dyn ToRequest<AL, BL, AA, BA>>,
    ) -> Result<(), rfc003::insert_state::Error>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
    {
        let swap_seed = self.seed.swap_seed(id);
        let swap_request = partial_swap_request.to_request(id, &swap_seed);

        self.insert_state_into_stores(bob_dial_info.peer_id.clone(), swap_request.clone())?;

        let future = {
            let swarm = Arc::clone(&self.swarm);
            let state_store = Arc::clone(&self.state_store);
            let cloned_self = self.clone();

            async move {
                let response = swarm
                    .send_rfc003_swap_request(bob_dial_info.clone(), swap_request.clone())
                    .compat()
                    .await
                    .map_err(|e| {
                        log::error!(
                            "Failed to send swap request to {} because {:?}",
                            bob_dial_info.peer_id,
                            e
                        );
                    })?;

                let alice_state = match response.clone() {
                    Ok(accept) => State::accepted(swap_request.clone(), accept, swap_seed),
                    Err(decline) => State::declined(swap_request.clone(), decline, swap_seed),
                };
                state_store.insert(id, alice_state.clone());

                cloned_self.spawn(swap_request, response);

                Ok(())
            }
        };

        tokio::spawn(future.boxed().compat());

        Ok(())
    }
}
