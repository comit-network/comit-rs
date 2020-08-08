use crate::{asset, network::OrderId, SharedSwapId};
use futures::prelude::*;
use libp2p::{
    core::upgrade,
    request_response::{ProtocolName, RequestResponseCodec},
};
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Debug, Clone, Copy)]
pub struct TakeOrderProtocol;

impl ProtocolName for TakeOrderProtocol {
    fn protocol_name(&self) -> &[u8] {
        b"/comit/take-order/1.0.0"
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TakeOrderCodec;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Request {
    pub(crate) order_id: OrderId,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    pub(crate) amount: asset::Bitcoin,
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
pub struct Confirmation {
    pub shared_swap_id: SharedSwapId,
    // TODO: We should store this locally, the context of the substream should be good enough to
    // know which order was confirmed, no need to send this across the wire again. See "context" in
    // announce protocol.
    pub order_id: OrderId,
}

#[async_trait::async_trait]
impl RequestResponseCodec for TakeOrderCodec {
    type Protocol = TakeOrderProtocol;
    type Request = Request;
    type Response = Confirmation;

    /// Reads a take order request from the given I/O stream.
    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let message = upgrade::read_one(io, 1024)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut de = serde_json::Deserializer::from_slice(&message);
        let req = Request::deserialize(&mut de)?;

        Ok(req)
    }

    /// Reads a response (to a take order request) from the given I/O stream.
    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let message = upgrade::read_one(io, 1024)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut de = serde_json::Deserializer::from_slice(&message);
        let res = Confirmation::deserialize(&mut de)?;

        Ok(res)
    }

    /// Writes a take order request to the given I/O stream.
    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let bytes = serde_json::to_vec(&req)?;
        upgrade::write_one(io, &bytes).await?;

        Ok(())
    }

    /// Writes a response (to a take order request) to the given I/O stream.
    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let bytes = serde_json::to_vec(&res)?;
        upgrade::write_one(io, &bytes).await?;

        Ok(())
    }
}
