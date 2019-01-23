use crate::{
    comit_client,
    swap_protocols::{
        asset::Asset,
        rfc003::{
            self,
            events::{CommunicationEvents, ResponseFuture, StateMachineResponseFuture},
            ledger::Ledger,
            Alice,
        },
    },
};
use futures::Future;
use std::sync::Arc;

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

impl<C: comit_client::Client, AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>
    CommunicationEvents<Alice<AL, BL, AA, BA>> for AliceToBob<C, AL, BL>
{
    fn request_responded(
        &mut self,
        request: &comit_client::rfc003::Request<AL, BL, AA, BA>,
    ) -> &mut ResponseFuture<Alice<AL, BL, AA, BA>> {
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
