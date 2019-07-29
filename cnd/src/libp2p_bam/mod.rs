mod behaviour;
mod handler;
mod protocol;
mod substream;

use crate::libp2p_bam::handler::InnerEvent;

pub use self::{
    behaviour::{BamBehaviour, BehaviourInEvent, BehaviourOutEvent},
    handler::{BamHandler, PendingIncomingRequest, PendingOutgoingRequest},
    protocol::{BamProtocol, BamStream},
};

use libp2p::core::protocols_handler::ProtocolsHandlerEvent;

pub type BamHandlerEvent = ProtocolsHandlerEvent<BamProtocol, PendingOutgoingRequest, InnerEvent>;
