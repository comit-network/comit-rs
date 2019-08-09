use crate::libp2p_bam::{
    protocol::BamProtocol,
    substream::{self, Advance, Advanced},
    BamHandlerEvent,
};
use bam::{
    json::{Frame, FrameType, JsonFrameCodec, OutboundRequest, Response, ValidatedInboundRequest},
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
    inbound_substreams: Vec<substream::inbound::State<TSubstream>>,
    #[derivative(Debug = "ignore")]
    outbound_substreams: Vec<substream::outbound::State<TSubstream>>,

    #[derivative(Debug = "ignore")]
    current_task: Option<Task>,

    known_headers: HashMap<String, HashSet<String>>,
}

#[derive(Debug)]
pub enum Error {
    Stream(bam::json::CodecError),
    DroppedResponseSender(Canceled),
}

impl From<Canceled> for Error {
    fn from(e: Canceled) -> Self {
        Error::DroppedResponseSender(e)
    }
}

impl From<bam::json::CodecError> for Error {
    fn from(e: bam::json::CodecError) -> Self {
        Error::Stream(e)
    }
}

impl<TSubstream> BamHandler<TSubstream> {
    pub fn new(known_headers: HashMap<String, HashSet<String>>) -> Self {
        Self {
            known_headers,
            inbound_substreams: Vec::new(),
            outbound_substreams: Vec::new(),
            current_task: None,
        }
    }
}

#[derive(Debug)]
pub struct PendingOutboundRequest {
    pub request: OutboundRequest,
    pub channel: oneshot::Sender<Response>,
}

#[derive(Debug)]
pub struct PendingInboundRequest {
    pub request: ValidatedInboundRequest,
    pub channel: oneshot::Sender<Response>,
}

#[derive(Debug)]
pub struct PendingInboundResponse {
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
    PendingOutboundRequest { request: PendingOutboundRequest },
}

/// Events that occur 'out'side of this node i.e. events from a peer node.
#[derive(Debug)]
pub enum ProtocolOutEvent {
    InboundRequest(PendingInboundRequest),
    InboundResponse(PendingInboundResponse),
    BadInboundRequest(AutomaticallyGeneratedErrorResponse),
    BadInboundResponse,
    UnexpectedFrameType {
        bad_frame: Frame,
        expected_type: FrameType,
    },
    UnexpectedEOF,
    Error {
        error: Error,
    },
}

/// Different kinds of `OutboundOpenInfo` that we may want to pass when emitted
/// an instance of `ProtocolsHandlerEvent::OutboundSubstreamRequest`.
#[derive(Debug)]
pub enum ProtocolOutboundOpenInfo {
    PendingOutboundRequest { request: PendingOutboundRequest },
}

impl<TSubstream: AsyncRead + AsyncWrite> ProtocolsHandler for BamHandler<TSubstream> {
    type InEvent = ProtocolInEvent;
    type OutEvent = ProtocolOutEvent;
    type Error = bam::json::CodecError;
    type Substream = TSubstream;
    type InboundProtocol = BamProtocol;
    type OutboundProtocol = BamProtocol;
    type OutboundOpenInfo = ProtocolOutboundOpenInfo;

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol> {
        SubstreamProtocol::new(BamProtocol {})
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        stream: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
    ) {
        self.inbound_substreams
            .push(substream::inbound::State::WaitingMessage { stream });

        if let Some(task) = &self.current_task {
            task.notify()
        }
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        stream: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
        outbound_open_info: Self::OutboundOpenInfo,
    ) {
        match outbound_open_info {
            ProtocolOutboundOpenInfo::PendingOutboundRequest {
                request: PendingOutboundRequest { request, channel },
            } => {
                self.outbound_substreams
                    .push(substream::outbound::State::WaitingSend {
                        frame: request.into_frame(),
                        response_sender: channel,
                        stream,
                    });
            }
        }

        if let Some(task) = &self.current_task {
            task.notify()
        }
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        match event {
            ProtocolInEvent::PendingOutboundRequest { request } => {
                self.outbound_substreams
                    .push(substream::outbound::State::WaitingOpen { request });
            }
        }

        if let Some(task) = &self.current_task {
            task.notify()
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
        if let Some(result) = poll_substreams(&mut self.outbound_substreams, &self.known_headers) {
            return result;
        }

        if let Some(result) = poll_substreams(&mut self.inbound_substreams, &self.known_headers) {
            return result;
        }

        self.current_task = Some(futures::task::current());

        Ok(Async::NotReady)
    }
}

fn poll_substreams<S: Display + Advance>(
    substreams: &mut Vec<S>,
    known_headers: &HashMap<String, HashSet<String>>,
) -> Option<Poll<BamHandlerEvent, bam::json::CodecError>> {
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
