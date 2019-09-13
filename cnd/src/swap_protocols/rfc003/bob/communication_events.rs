use comit::{
    asset::Asset,
    rfc003::{
        self,
        events::{CommunicationEvents, ResponseFuture},
        Ledger,
    },
};
use debug_stub_derive::DebugStub;

#[derive(DebugStub)]
pub struct BobToAlice<AL: Ledger, BL: Ledger> {
    #[debug_stub = "ResponseFuture"]
    response_future: Box<ResponseFuture<AL, BL>>,
}

impl<AL: Ledger, BL: Ledger> BobToAlice<AL, BL> {
    pub fn new(response_future: Box<ResponseFuture<AL, BL>>) -> Self {
        Self { response_future }
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> CommunicationEvents<AL, BL, AA, BA>
    for BobToAlice<AL, BL>
{
    fn request_responded(
        &mut self,
        _request: &rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> &mut ResponseFuture<AL, BL> {
        &mut self.response_future
    }
}
