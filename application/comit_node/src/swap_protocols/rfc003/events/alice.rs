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
pub struct AliceToBob<C, AL: Ledger, BL: Ledger> {
    #[allow(clippy::type_complexity)]
    response_future:
        Option<Box<StateMachineResponseFuture<AL::Identity, BL::Identity, BL::LockDuration>>>,
    client: Arc<C>,
}

impl<C, AL: Ledger, BL: Ledger> AliceToBob<C, AL, BL> {
    pub fn new(client: Arc<C>) -> Self {
        AliceToBob {
            client,
            response_future: None,
        }
    }
}

impl<C: comit_client::Client, AL: Ledger, BL: Ledger, SA: Asset, TA: Asset>
    CommunicationEvents<Alice<AL, BL, SA, TA>> for AliceToBob<C, AL, BL>
{
    fn request_responded(
        &mut self,
        request: &comit_client::rfc003::Request<AL, BL, SA, TA>,
    ) -> &mut ResponseFuture<Alice<AL, BL, SA, TA>> {
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
