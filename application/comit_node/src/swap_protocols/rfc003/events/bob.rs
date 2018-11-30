use comit_client;
use swap_protocols::{
    asset::Asset,
    rfc003::{
        events::{CommunicationEvents, ResponseFuture},
        ledger::Ledger,
        roles::Bob,
    },
};

#[derive(DebugStub)]
pub struct BobToAlice<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset> {
    #[debug_stub = "ResponseFuture"]
    response_future: Box<ResponseFuture<Bob<SL, TL, SA, TA>>>,
}

impl<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset> BobToAlice<SL, TL, SA, TA> {
    pub fn new(response_future: Box<ResponseFuture<Bob<SL, TL, SA, TA>>>) -> Self {
        Self { response_future }
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
