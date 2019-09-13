use crate::{asset::Asset, rfc003};
use futures::Future;
use libp2p_core::{Multiaddr, PeerId};
use std::io;

pub trait Client: Send + Sync + 'static {
    fn send_rfc003_swap_request<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset>(
        &self,
        dial_information: (PeerId, Option<Multiaddr>),
        request: rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> Box<
        dyn Future<
                Item = Result<
                    crate::rfc003::messages::AcceptResponseBody<AL, BL>,
                    crate::rfc003::messages::DeclineResponseBody,
                >,
                Error = RequestError,
            > + Send,
    >;
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
