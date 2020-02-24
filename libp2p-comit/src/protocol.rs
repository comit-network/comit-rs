use crate::frame::{self, JsonFrameCodec};
use futures::future;
use futures_codec::Framed;
use libp2p::{
    core::{InboundUpgrade, OutboundUpgrade, UpgradeInfo},
    swarm::NegotiatedSubstream,
};
use std::{convert::Infallible, iter};

pub type Frames = Framed<NegotiatedSubstream, JsonFrameCodec>;

#[derive(Clone, Copy, Debug)]
pub struct ComitProtocolConfig {}

impl UpgradeInfo for ComitProtocolConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/comit/1.0.0")
    }
}

impl InboundUpgrade<NegotiatedSubstream> for ComitProtocolConfig {
    type Output = Frames;
    type Error = Infallible;
    type Future = future::Ready<Result<Self::Output, Infallible>>;

    #[inline]
    fn upgrade_inbound(self, socket: NegotiatedSubstream, _: Self::Info) -> Self::Future {
        let codec = frame::JsonFrameCodec::default();
        let framed = Framed::new(socket, codec);

        future::ok(framed)
    }
}

impl OutboundUpgrade<NegotiatedSubstream> for ComitProtocolConfig {
    type Output = Frames;
    type Error = Infallible;
    type Future = future::Ready<Result<Self::Output, Infallible>>;

    #[inline]
    fn upgrade_outbound(self, socket: NegotiatedSubstream, _: Self::Info) -> Self::Future {
        let codec = frame::JsonFrameCodec::default();
        let framed = Framed::new(socket, codec);

        future::ok(framed)
    }
}
