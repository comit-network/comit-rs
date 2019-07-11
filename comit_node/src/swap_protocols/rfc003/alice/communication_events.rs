use crate::{
    comit_client,
    network::DialInformation,
    swap_protocols::{
        asset::Asset,
        rfc003::{
            self,
            events::{CommunicationEvents, ResponseFuture},
            ledger::Ledger,
        },
    },
};
use futures::Future;
use std::sync::Arc;

#[allow(missing_debug_implementations)]
pub struct AliceToBob<C, AL: Ledger, BL: Ledger> {
    #[allow(clippy::type_complexity)]
    response_future: Option<Box<ResponseFuture<AL, BL>>>,
    client: Arc<C>,
    bob_dial_info: DialInformation,
}

impl<C, AL: Ledger, BL: Ledger> AliceToBob<C, AL, BL> {
    pub fn new(client: Arc<C>, bob_dial_info: DialInformation) -> Self {
        AliceToBob {
            client,
            bob_dial_info,
            response_future: None,
        }
    }
}

impl<C: comit_client::Client, AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>
    CommunicationEvents<AL, BL, AA, BA> for AliceToBob<C, AL, BL>
{
    fn request_responded(
        &mut self,
        request: &rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> &mut ResponseFuture<AL, BL> {
        let bob_id = self.bob_dial_info.clone();
        let (client, response_future) = (self.client.as_ref(), &mut self.response_future);
        response_future.get_or_insert_with(|| {
            Box::new(
                client
                    .send_rfc003_swap_request(bob_id, request.clone())
                    .map_err(rfc003::Error::SwapResponse)
                    .map(|result| result.map(Into::into)),
            )
        })
    }
}
