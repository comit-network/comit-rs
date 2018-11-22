use comit_client::{self, SwapReject};
use futures::{sync::oneshot, Future};
use std::sync::Arc;
use swap_protocols::{
    asset::Asset,
    rfc003::{
        bob::PendingResponses,
        events::{CommunicationEvents, ResponseFuture},
        ledger::Ledger,
        roles::Bob,
        state_machine::StateMachineResponse,
    },
};
use swaps::common::SwapId;

#[derive(DebugStub)]
pub struct BobToAlice<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset> {
    #[debug_stub = "ResponseFuture"]
    response_future: Box<ResponseFuture<Bob<SL, TL, SA, TA>>>,
}

impl<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset> BobToAlice<SL, TL, SA, TA> {
    pub fn new(
        pending_responses: Arc<PendingResponses<SwapId>>,
        current_swap: SwapId,
        response_sender: oneshot::Sender<
            Result<
                StateMachineResponse<SL::HtlcIdentity, TL::HtlcIdentity, TL::LockDuration>,
                SwapReject,
            >,
        >,
    ) -> Self {
        Self {
            response_future: {
                let future = pending_responses
                    .create::<SL, TL, SA, TA>(current_swap)
                    .and_then(|response| {
                        response_sender
                            .send(response.clone())
                            .expect("receiver should never go out of scope");

                        Ok(response)
                    });

                Box::new(future)
            },
        }
    }
}

impl<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset> CommunicationEvents<Bob<SL, TL, SA, TA>>
    for BobToAlice<SL, TL, SA, TA>
{
    fn request_responded(
        &mut self,
        _request: &comit_client::rfc003::Request<SL, TL, SA, TA>,
    ) -> &mut ResponseFuture<Bob<SL, TL, SA, TA>> {
        &mut self.response_future
    }
}
