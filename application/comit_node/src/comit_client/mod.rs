pub mod bam;
pub mod fake;

use futures::Future;
use std::{io, net::SocketAddr, sync::Arc};

use std::{fmt::Debug, panic::RefUnwindSafe};
use swap_protocols::{ledger::Ledger, rfc003, wire_types};

pub trait Client {
    fn send_swap_request<
        SL: Ledger,
        TL: Ledger,
        SA: Into<wire_types::Asset>,
        TA: Into<wire_types::Asset>,
    >(
        &self,
        request: rfc003::Request<SL, TL, SA, TA>,
    ) -> Box<
        Future<
                Item = Result<rfc003::AcceptResponse<SL, TL>, SwapReject>,
                Error = SwapResponseError,
            > + Send,
    >;
}

pub trait ClientFactory<C: Client>: Send + Sync + RefUnwindSafe + Debug {
    fn client_for(&self, comit_node_socket_addr: SocketAddr) -> Result<Arc<C>, ClientFactoryError>;
}

#[derive(Clone, Debug)]
pub enum SwapReject {
    /// The counterparty rejected the request
    Rejected,
}

#[derive(Debug)]
pub enum SwapResponseError {
    /// The counterparty had an internal error while processing the request
    InternalError,
    /// The counterparty produced a response that caused at the transport level
    TransportError,
    /// The counterparty produced an invalid response to the request
    InvalidResponse,
}

#[derive(Debug)]
pub enum ClientFactoryError {
    Connection(io::Error),
}

impl From<io::Error> for ClientFactoryError {
    fn from(e: io::Error) -> Self {
        ClientFactoryError::Connection(e)
    }
}
