use crate::libp2p_bam::{handler::InnerEvent, protocol::BamStream, BamHandlerEvent};

use libp2p::core::protocols_handler::ProtocolsHandlerEvent;
use std::collections::{HashMap, HashSet};

pub mod r#in;
pub mod out;

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
use crate::libp2p_bam::handler::Error;

impl<S: CloseStream> Advanced<S> {
    fn error<E: Into<Error>>(stream: BamStream<S::TSubstream>, error: E) -> Self {
        let error = error.into();

        Self {
            new_state: Some(S::close(stream)),
            event: Some(ProtocolsHandlerEvent::Custom(InnerEvent::Error { error })),
        }
    }
}

pub trait CloseStream: Sized {
    type TSubstream;

    fn close(stream: BamStream<Self::TSubstream>) -> Self;
}
