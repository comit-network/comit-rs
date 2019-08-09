use crate::libp2p_bam::{
    handler::{Error, ProtocolOutEvent},
    protocol::BamStream,
    BamHandlerEvent,
};
use bam::{
    frame::{ErrorType, Header, UnknownMandatoryHeaders},
    Frame,
};
use libp2p::core::protocols_handler::ProtocolsHandlerEvent;
use std::collections::{HashMap, HashSet};

pub mod inbound;
pub mod outbound;

#[allow(missing_debug_implementations)]
pub struct Advanced<S> {
    /// The optional new state we transitioned to.
    pub new_state: Option<S>,
    /// The optional event we generated as part of the transition.
    pub event: Option<BamHandlerEvent>,
}

pub trait Advance: Sized {
    fn advance(self, known_headers: &HashMap<String, HashSet<String>>) -> Advanced<Self>;
}

impl<S> Advanced<S> {
    fn transition_to(new_state: S) -> Self {
        Self {
            new_state: Some(new_state),
            event: None,
        }
    }

    fn emit_event(event: BamHandlerEvent) -> Self {
        Self {
            new_state: None,
            event: Some(event),
        }
    }

    fn end() -> Self {
        Self {
            new_state: None,
            event: None,
        }
    }
}

impl<S: CloseStream> Advanced<S> {
    fn error<E: Into<Error>>(stream: BamStream<S::TSubstream>, error: E) -> Self {
        let error = error.into();

        Self {
            new_state: Some(S::close(stream)),
            event: Some(ProtocolsHandlerEvent::Custom(ProtocolOutEvent::Error {
                error,
            })),
        }
    }
}

pub trait CloseStream: Sized {
    type TSubstream;

    fn close(stream: BamStream<Self::TSubstream>) -> Self;
}

pub fn malformed_frame_error(error: serde_json::Error) -> bam::frame::Error {
    log::warn!(target: "sub-libp2p", "incoming request was malformed: {:?}", error);

    bam::frame::Error::new(ErrorType::MalformedFrame)
}

pub fn unknown_request_type_error(request_type: &str) -> bam::frame::Error {
    log::warn!(target: "sub-libp2p", "request type '{}' is unknown", request_type);

    bam::frame::Error::new(ErrorType::UnknownRequestType)
}

pub fn unknown_mandatory_header_error(
    unknown_headers: UnknownMandatoryHeaders,
) -> bam::frame::Error {
    bam::frame::Error::new(ErrorType::UnknownMandatoryHeader).with_details(
        Header::with_value(unknown_headers)
            .expect("list of strings should serialize to serde_json::Value"),
    )
}

pub fn unknown_frame_type_error(bad_frame: Frame) -> bam::frame::Error {
    log::error!(target: "sub-libp2p", "unknown type for frame {:?}", bad_frame);

    bam::frame::Error::new(ErrorType::UnknownFrameType)
}
