use crate::{
    network::DialInformation,
    swap_protocols::{
        self,
        asset::Asset,
        dependencies::LedgerEventDependencies,
        rfc003::{self, create_ledger_events::CreateLedgerEvents},
    },
};
use futures::Future;
use std::io;

pub trait Client: Send + Sync + 'static {
    fn send_rfc003_swap_request<
        AL: swap_protocols::rfc003::Ledger,
        BL: swap_protocols::rfc003::Ledger,
        AA: Asset,
        BA: Asset,
    >(
        &self,
        peer_identity: DialInformation,
        request: swap_protocols::rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> Box<dyn Future<Item = rfc003::Response<AL, BL>, Error = RequestError> + Send>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum RequestError {
    /// The other node had an internal error while processing the request
    InternalError,
    /// The other node produced an invalid response
    InvalidResponse,
    /// We had to establish a new connection to make the request but it failed
    Connecting(io::ErrorKind),
    /// We were unable to send the data on the existing connection
    Connection,
}
