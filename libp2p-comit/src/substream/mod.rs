use crate::{handler::Error, protocol::Frames, ComitHandlerEvent};
use libp2p::swarm::ProtocolsHandlerEvent;
use std::{
    collections::{HashMap, HashSet},
    pin::Pin,
    task::Context,
};

pub mod inbound;
pub mod outbound;

#[allow(missing_debug_implementations)]
pub struct Advanced<S> {
    /// The optional new state we transitioned to.
    pub new_state: Option<S>,
    /// The optional event we generated as part of the transition.
    pub event: Option<ComitHandlerEvent>,
}

pub trait Advance: Sized {
    fn advance(
        self,
        known_headers: &HashMap<String, HashSet<String>>,
        cx: &mut Context<'_>,
    ) -> Advanced<Self>;
}

impl<S> Advanced<S> {
    fn transition_to(new_state: S) -> Self {
        Self {
            new_state: Some(new_state),
            event: None,
        }
    }

    fn end() -> Self {
        Self {
            new_state: None,
            event: None,
        }
    }
}

impl<S> Advanced<S>
where
    S: CloseStream,
{
    fn error<E>(stream: Pin<Box<Frames>>, error: E) -> Self
    where
        E: Into<Error>,
    {
        let error = error.into();

        Self {
            new_state: Some(S::close(stream)),
            event: Some(ProtocolsHandlerEvent::Close(error)),
        }
    }
}

pub trait CloseStream: Sized {
    fn close(stream: Pin<Box<Frames>>) -> Self;
}
