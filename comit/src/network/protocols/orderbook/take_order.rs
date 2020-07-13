use crate::{network::OrderId, SharedSwapId};
use futures::{prelude::*, AsyncWriteExt};
use libp2p::{
    core::upgrade,
    request_response::{ProtocolName, RequestResponseCodec},
};
use serde::{Deserialize, Serialize};
use std::io;
use tracing::debug;

#[derive(Debug, Clone, Copy)]
pub struct TakeOrderProtocol;

impl ProtocolName for TakeOrderProtocol {
    fn protocol_name(&self) -> &[u8] {
        b"/comit/orderbook/take/1.0.0"
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TakeOrderCodec;

/// The different responses we can send back as part of an announcement.
///
/// For now, this only includes a generic error variant in addition to the
/// confirmation because we simply close the connection in case of an error.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Response {
    Confirmation {
        order_id: OrderId,
        shared_swap_id: SharedSwapId,
    },
    Error,
}

#[async_trait::async_trait]
impl RequestResponseCodec for TakeOrderCodec {
    type Protocol = TakeOrderProtocol;
    type Request = OrderId;
    type Response = Response;

    // handling take order req
    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let message = upgrade::read_one(io, 1024)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut de = serde_json::Deserializer::from_slice(&message);
        let order_id = OrderId::deserialize(&mut de)?;
        debug!("read request order id: {}", order_id);

        Ok(order_id)
    }

    // handling take order resp
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
        let res = Response::deserialize(&mut de)?;

        Ok(res)
    }

    // sending take order req
    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!("writing request order id: {}", req);
        let bytes = serde_json::to_vec(&req)?;
        upgrade::write_one(io, &bytes).await?;

        Ok(())
    }

    // sending take order resp
    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        match res {
            Response::Confirmation { .. } => {
                let bytes = serde_json::to_vec(&res)?;
                upgrade::write_one(io, &bytes).await?;
            }
            Response::Error => {
                debug!("closing write response channel");
                // for now, errors just close the substream.
                // we can send actual error responses at a later point
                // denied take order request is an error
                let _ = io.close().await;
            }
        }

        Ok(())
    }
}
