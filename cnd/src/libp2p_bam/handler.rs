use crate::libp2p_bam::{
    protocol::BamProtocol,
    substream::{self, Advance, Advanced},
    BamHandlerEvent,
};
use bam::{
    json::{Frame, FrameType, JsonFrameCodec, OutgoingRequest, Response, ValidatedIncomingRequest},
    IntoFrame,
};
use derivative::Derivative;
use futures::{
    sync::oneshot::{self, Canceled},
    task::Task,
    Async, Poll,
};
use libp2p::core::{
    protocols_handler::{KeepAlive, ProtocolsHandler, ProtocolsHandlerUpgrErr, SubstreamProtocol},
    upgrade::Negotiated,
};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    fmt::Display,
};
use tokio::{
    codec::Framed,
    prelude::{AsyncRead, AsyncWrite},
};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct BamHandler<TSubstream> {
    #[derivative(Debug = "ignore")]
    incoming_substreams: Vec<substream::r#in::State<TSubstream>>,
    #[derivative(Debug = "ignore")]
    outgoing_substreams: Vec<substream::out::State<TSubstream>>,

    #[derivative(Debug = "ignore")]
    current_task: Option<Task>,

    known_headers: HashMap<String, HashSet<String>>,
}

#[derive(Debug)]
pub enum Error {
    Stream(bam::json::Error),
    DroppedResponseSender(Canceled),
}

impl From<Canceled> for Error {
    fn from(e: Canceled) -> Self {
        Error::DroppedResponseSender(e)
    }
}

impl From<bam::json::Error> for Error {
    fn from(e: bam::json::Error) -> Self {
        Error::Stream(e)
    }
}

impl<TSubstream> BamHandler<TSubstream> {
    pub fn new(known_headers: HashMap<String, HashSet<String>>) -> Self {
        Self {
            known_headers,
            incoming_substreams: Vec::new(),
            outgoing_substreams: Vec::new(),
            current_task: None,
        }
    }
}

#[derive(Debug)]
pub struct PendingOutgoingRequest {
    pub request: OutgoingRequest,
    pub channel: oneshot::Sender<Response>,
}

#[derive(Debug)]
pub struct PendingIncomingRequest {
    pub request: ValidatedIncomingRequest,
    pub channel: oneshot::Sender<Response>,
}

#[derive(Debug)]
pub struct PendingIncomingResponse {
    pub response: Response,
    pub channel: oneshot::Sender<Response>,
}

#[derive(Debug)]
pub struct AutomaticallyGeneratedErrorResponse {
    pub response: Response,
    pub channel: oneshot::Sender<Response>,
}

/// Events that occur 'in' this node (as opposed to events from a peer node).
#[derive(Debug)]
pub enum ProtocolInEvent {
    PendingOutgoingRequest { request: PendingOutgoingRequest },
}

/// Events that occur 'out'side of this node i.e. events from a peer node.
#[derive(Debug)]
pub enum ProtocolOutEvent {
    IncomingRequest(PendingIncomingRequest),
    IncomingResponse(PendingIncomingResponse),
    BadIncomingRequest(AutomaticallyGeneratedErrorResponse),
    BadIncomingResponse,
    UnexpectedFrameType {
        bad_frame: Frame,
        expected_type: FrameType,
    },
    UnexpectedEOF,
    Error {
        error: Error,
    },
}

impl<TSubstream: AsyncRead + AsyncWrite> ProtocolsHandler for BamHandler<TSubstream> {
    type InEvent = ProtocolInEvent;
    type OutEvent = ProtocolOutEvent;
    type Error = bam::json::Error;
    type Substream = TSubstream;
    type InboundProtocol = BamProtocol;
    type OutboundProtocol = BamProtocol;
    type OutboundOpenInfo = PendingOutgoingRequest;

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol> {
        SubstreamProtocol::new(BamProtocol {})
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        stream: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
    ) {
        self.incoming_substreams
            .push(substream::r#in::State::WaitingMessage { stream });

        if let Some(task) = &self.current_task {
            task.notify()
        }
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        stream: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
        pending_incoming_request: Self::OutboundOpenInfo,
    ) {
        let PendingOutgoingRequest { request, channel } = pending_incoming_request;

        self.outgoing_substreams
            .push(substream::out::State::WaitingSend {
                msg: request.into_frame(),
                response_sender: channel,
                stream,
            });

        if let Some(task) = &self.current_task {
            task.notify()
        }
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        match event {
            ProtocolInEvent::PendingOutgoingRequest { request } => {
                self.outgoing_substreams
                    .push(substream::out::State::WaitingOpen { req: request });

                if let Some(task) = &self.current_task {
                    task.notify()
                }
            }
        }
    }

    fn inject_dial_upgrade_error(
        &mut self,
        _info: Self::OutboundOpenInfo,
        _error: ProtocolsHandlerUpgrErr<Infallible>,
    ) {
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::Yes
    }

    fn poll(&mut self) -> Poll<BamHandlerEvent, Self::Error> {
        if let Some(result) = poll_substreams(&mut self.outgoing_substreams, &self.known_headers) {
            return result;
        }

        if let Some(result) = poll_substreams(&mut self.incoming_substreams, &self.known_headers) {
            return result;
        }

        self.current_task = Some(futures::task::current());

        Ok(Async::NotReady)
    }
}

fn poll_substreams<S: Display + Advance>(
    substreams: &mut Vec<S>,
    known_headers: &HashMap<String, HashSet<String>>,
) -> Option<Poll<BamHandlerEvent, bam::json::Error>> {
    log::debug!("polling {} substreams", substreams.len());

    // We remove each element from `substreams` one by one and add them back.
    for n in (0..substreams.len()).rev() {
        let substream_state = substreams.swap_remove(n);

        let log_message = format!("transition from {}", substream_state);

        let Advanced { new_state, event } = substream_state.advance(known_headers);

        if let Some(new_state) = new_state {
            log::trace!(target: "sub-libp2p", "{} to {}", log_message, new_state);
            substreams.push(new_state);
        }

        if let Some(event) = event {
            log::trace!(target: "sub-libp2p", "emitting {:?}", event);
            return Some(Ok(Async::Ready(event)));
        }
    }
    None
}
