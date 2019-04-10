use crate::libp2p_bam::protocol::BamConfig;
use bam::json::{Frame, JsonFrameCodec};
use futures::sink::Sink;
use libp2p::core::{
    protocols_handler::{
        KeepAlive, ProtocolsHandler, ProtocolsHandlerEvent, ProtocolsHandlerUpgrErr,
    },
    upgrade::Negotiated,
};
use std::{collections::vec_deque::VecDeque, convert::Infallible, marker::PhantomData};
use tokio::{
    codec::Framed,
    prelude::{stream::Stream, Async, AsyncRead, AsyncSink, AsyncWrite},
};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct BamHandler<TSubstream> {
    marker: PhantomData<TSubstream>,
    #[derivative(Debug = "ignore")]
    framed: Option<Framed<Negotiated<TSubstream>, JsonFrameCodec>>,
    to_be_sent: VecDeque<Frame>,
}

impl<TSubstream> BamHandler<TSubstream> {
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
            framed: None,
            to_be_sent: VecDeque::new(),
        }
    }
}

impl<TSubstream: AsyncRead + AsyncWrite> ProtocolsHandler for BamHandler<TSubstream> {
    type InEvent = Frame;
    type OutEvent = Frame;
    type Error = bam::json::Error;
    type Substream = TSubstream;
    type InboundProtocol = BamConfig;
    type OutboundProtocol = BamConfig;
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> Self::InboundProtocol {
        BamConfig {}
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
    ) {
        self.framed = Some(protocol)
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        protocol: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
        _info: Self::OutboundOpenInfo,
    ) {
        self.framed = Some(protocol)
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        self.to_be_sent.push_back(event)
    }

    fn inject_dial_upgrade_error(
        &mut self,
        _info: Self::OutboundOpenInfo,
        _error: ProtocolsHandlerUpgrErr<Infallible>,
    ) {

    }

    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::Forever
    }

    fn poll(
        &mut self,
    ) -> Result<
        Async<
            ProtocolsHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::OutEvent>,
        >,
        Self::Error,
    > {
        match self.framed.as_mut() {
            Some(framed) => {
                match framed.poll_complete() {
                    Ok(Async::Ready(_)) => {}
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(e) => return Err(e),
                }

                if let Some(frame) = self.to_be_sent.pop_front() {
                    match framed.start_send(frame) {
                        Ok(AsyncSink::Ready) => {
                            // cool, we sent it!
                        }
                        Ok(AsyncSink::NotReady(pending_frame)) => {
                            self.to_be_sent.push_front(pending_frame)
                        }
                        Err(e) => return Err(e),
                    }
                }

                match framed.poll() {
                    Ok(Async::Ready(Some(frame))) => {
                        return Ok(Async::Ready(ProtocolsHandlerEvent::Custom(frame)))
                    }
                    Ok(Async::Ready(None)) => unimplemented!("TBD"),
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(e) => return Err(e),
                }
            }
            None => return Ok(Async::NotReady),
        }
    }
}
