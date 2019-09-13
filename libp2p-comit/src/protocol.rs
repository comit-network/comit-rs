use crate::frame::{self, JsonFrameCodec};
use futures::future::FutureResult;
use libp2p_core::{InboundUpgrade, Negotiated, OutboundUpgrade, UpgradeInfo};
use std::{convert::Infallible, iter};
use tokio::{
    codec::{Decoder, Framed},
    prelude::*,
};

pub type BamStream<TSubstream> = Framed<Negotiated<TSubstream>, JsonFrameCodec>;

#[derive(Clone, Copy, Debug)]
pub struct BamProtocol {}

impl UpgradeInfo for BamProtocol {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/bam/json/1.0.0")
    }
}

impl<TSubstream> InboundUpgrade<TSubstream> for BamProtocol
where
    TSubstream: AsyncRead + AsyncWrite,
{
    type Output = BamStream<TSubstream>;
    type Error = Infallible;
    type Future = FutureResult<Self::Output, Self::Error>;

    #[inline]
    fn upgrade_inbound(self, socket: Negotiated<TSubstream>, _: Self::Info) -> Self::Future {
        let codec = frame::JsonFrameCodec::default();
        futures::future::ok(codec.framed(socket))
    }
}

impl<TSubstream> OutboundUpgrade<TSubstream> for BamProtocol
where
    TSubstream: AsyncRead + AsyncWrite,
{
    type Output = BamStream<TSubstream>;
    type Error = Infallible;
    type Future = FutureResult<Self::Output, Self::Error>;

    #[inline]
    fn upgrade_outbound(self, socket: Negotiated<TSubstream>, _: Self::Info) -> Self::Future {
        let codec = frame::JsonFrameCodec::default();
        futures::future::ok(codec.framed(socket))
    }
}
