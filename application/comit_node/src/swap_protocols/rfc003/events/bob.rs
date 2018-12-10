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
pub struct BobToAlice<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    #[debug_stub = "ResponseFuture"]
    response_future: Box<ResponseFuture<Bob<AL, BL, AA, BA>>>,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> BobToAlice<AL, BL, AA, BA> {
    pub fn new(response_future: Box<ResponseFuture<Bob<AL, BL, AA, BA>>>) -> Self {
        Self { response_future }
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> CommunicationEvents<Bob<AL, BL, AA, BA>>
    for BobToAlice<AL, BL, AA, BA>
{
    fn request_responded(
        &mut self,
        _request: &comit_client::rfc003::Request<AL, BL, AA, BA>,
    ) -> &mut ResponseFuture<Bob<AL, BL, AA, BA>> {
        &mut self.response_future
    }
}
