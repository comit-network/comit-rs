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

pub type BamStream<TSubstream> = Framed<Negotiated<TSubstream>, JsonFrameCodec>;

#[derive(Clone, Debug)]
pub struct BamConfig {}

impl UpgradeInfo for BamConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/bam/1.0.0")
    }
}

impl<TSubstream> InboundUpgrade<TSubstream> for BamConfig
where
    TSubstream: AsyncRead + AsyncWrite,
{
    type Output = BamStream<TSubstream>;
    type Error = Infallible;
    type Future = FutureResult<Self::Output, Self::Error>;

    #[inline]
    fn upgrade_inbound(self, socket: Negotiated<TSubstream>, _: Self::Info) -> Self::Future {
        let codec = json::JsonFrameCodec::default();

        log::debug!("inbound connected upgraded to json-bam");

        futures::future::ok(codec.framed(socket))
    }
}

impl<TSubstream> OutboundUpgrade<TSubstream> for BamConfig
where
    TSubstream: AsyncRead + AsyncWrite,
{
    type Output = BamStream<TSubstream>;
    type Error = Infallible;
    type Future = FutureResult<Self::Output, Self::Error>;

    #[inline]
    fn upgrade_outbound(self, socket: Negotiated<TSubstream>, _: Self::Info) -> Self::Future {
        let codec = json::JsonFrameCodec::default();

        log::debug!("outbound connected upgraded to json-bam");

        futures::future::ok(codec.framed(socket))
    }
}
