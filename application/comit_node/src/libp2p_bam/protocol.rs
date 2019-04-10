use bam::json::{self, JsonFrameCodec};
use futures::future::FutureResult;
use libp2p::{
    core::{upgrade::Negotiated, InboundUpgrade, UpgradeInfo},
    OutboundUpgrade,
};
use std::{convert::Infallible, iter};
use tokio::{
    codec::{Decoder, Framed},
    prelude::*,
};

#[derive(Clone, Debug)]
pub struct BamConfig {}

impl UpgradeInfo for BamConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/bam/1.0.0")
    }
}

impl<TSocket> InboundUpgrade<TSocket> for BamConfig
where
    TSocket: AsyncRead + AsyncWrite,
{
    type Output = Framed<Negotiated<TSocket>, JsonFrameCodec>;
    type Error = Infallible;
    type Future = FutureResult<Self::Output, Self::Error>;

    #[inline]
    fn upgrade_inbound(self, socket: Negotiated<TSocket>, _: Self::Info) -> Self::Future {
        let codec = json::JsonFrameCodec::default();

        futures::future::ok(codec.framed(socket))
    }
}

impl<TSocket> OutboundUpgrade<TSocket> for BamConfig
where
    TSocket: AsyncRead + AsyncWrite,
{
    type Output = Framed<Negotiated<TSocket>, JsonFrameCodec>;
    type Error = Infallible;
    type Future = FutureResult<Self::Output, Self::Error>;

    #[inline]
    fn upgrade_outbound(self, socket: Negotiated<TSocket>, _: Self::Info) -> Self::Future {
        let codec = json::JsonFrameCodec::default();

        futures::future::ok(codec.framed(socket))
    }
}
