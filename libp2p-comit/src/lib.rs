#![warn(
    unused_extern_crates,
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::fallible_impl_from,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::print_stdout,
    clippy::dbg_macro
)]
#![forbid(unsafe_code)]

#[macro_use]
pub mod frame;
mod behaviour;
pub mod handler;
mod protocol;
mod substream;

use serde::{Deserialize, Serialize};
use serde_json::{self, Value as JsonValue};

pub use self::{
    behaviour::{BehaviourOutEvent, Comit},
    handler::{ComitHandler, PendingInboundRequest, PendingOutboundRequest},
    protocol::{ComitProtocolConfig, Frames},
};
use crate::handler::{ProtocolOutEvent, ProtocolOutboundOpenInfo};
use libp2p::swarm::ProtocolsHandlerEvent;

pub type ComitHandlerEvent = ProtocolsHandlerEvent<
    ComitProtocolConfig,
    ProtocolOutboundOpenInfo,
    ProtocolOutEvent,
    handler::Error,
>;

pub trait IntoFrame<F> {
    fn into_frame(self) -> F;
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct Frame {
    #[serde(rename = "type")]
    pub frame_type: FrameType,
    pub payload: JsonValue,
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum FrameType {
    Request,
    Response,

    // This is currently the fallback to not fail on serialisation if the frame type is unknown
    // Unfortunately serde does not support deserialization into a String when using other
    #[serde(other)]
    Unknown,
}

impl Frame {
    pub fn new(frame_type: FrameType, payload: JsonValue) -> Self {
        Self {
            frame_type,
            payload,
        }
    }
}
