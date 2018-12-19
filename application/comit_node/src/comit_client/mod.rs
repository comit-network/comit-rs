pub mod bam;
pub mod fake;
pub mod rfc003;

use crate::swap_protocols::{self, asset::Asset};
use futures::Future;
use std::{fmt::Debug, io, net::SocketAddr, panic::RefUnwindSafe, sync::Arc};

pub trait Client: Send + Sync + 'static {
    fn send_swap_request<
        AL: swap_protocols::rfc003::Ledger,
        BL: swap_protocols::rfc003::Ledger,
        AA: Asset,
        BA: Asset,
    >(
        &self,
        request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Box<
        dyn Future<
                Item = Result<rfc003::AcceptResponseBody<AL, BL>, SwapReject>,
                Error = SwapResponseError,
            > + Send,
    >;
}

pub trait ClientFactory<C: Client>: Send + Sync + RefUnwindSafe + Debug {
    fn client_for(&self, comit_node_socket_addr: SocketAddr) -> Result<Arc<C>, ClientFactoryError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SwapReject {
    Declined { reason: Option<SwapDeclineReason> },
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapDeclineReason {
    BadRate,
}

#[derive(Clone, Debug, PartialEq)]
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
