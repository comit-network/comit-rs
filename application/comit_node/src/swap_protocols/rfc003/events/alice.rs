use comit_client;
use futures::Future;
use std::sync::Arc;
use swap_protocols::rfc003::roles::Alice;

use swap_protocols::{
    asset::Asset,
    rfc003::{
        self,
        events::{CommunicationEvents, ResponseFuture, StateMachineResponseFuture},
        ledger::Ledger,
    },
};

#[allow(missing_debug_implementations)]
pub struct AliceToBob<C, SL: Ledger, TL: Ledger> {
    #[allow(clippy::type_complexity)]
    response_future:
        Option<Box<StateMachineResponseFuture<SL::Identity, TL::Identity, TL::LockDuration>>>,
    client: Arc<C>,
}

impl<C, SL: Ledger, TL: Ledger> AliceToBob<C, SL, TL> {
    pub fn new(client: Arc<C>) -> Self {
        AliceToBob {
            client,
            response_future: None,
        }
    }
}

impl<C: comit_client::Client, SL: Ledger, TL: Ledger, SA: Asset, TA: Asset>
    CommunicationEvents<Alice<SL, TL, SA, TA>> for AliceToBob<C, SL, TL>
{
    fn request_responded(
        &mut self,
        request: &comit_client::rfc003::Request<SL, TL, SA, TA>,
    ) -> &mut ResponseFuture<Alice<SL, TL, SA, TA>> {
        let client = Arc::clone(&self.client);
        self.response_future.get_or_insert_with(|| {
            Box::new(
                client
                    .send_swap_request(request.clone())
                    .map_err(rfc003::Error::SwapResponse)
                    .map(|result| result.map(Into::into)),
            )
        })
    }
}
