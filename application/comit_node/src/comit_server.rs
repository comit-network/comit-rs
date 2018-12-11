use crate::{
    bam_api::rfc003::swap_config,
    swap_protocols::{rfc003, SwapId},
};
use bam::{connection::Connection, json};
use futures::{
    sync::{mpsc, oneshot},
    Future, Stream,
};
use std::{io, net::SocketAddr};
use tokio::{self, net::TcpListener};

impl ComitServer {
    pub fn new(
        sender: mpsc::UnboundedSender<(
            SwapId,
            rfc003::bob::SwapRequestKind,
            oneshot::Sender<rfc003::bob::SwapResponseKind>,
        )>,
    ) -> Self {
        Self { sender }
    }

    pub fn listen(self, addr: SocketAddr) -> impl Future<Item = (), Error = io::Error> {
        info!("ComitServer listening at {:?}", addr);
        let socket = TcpListener::bind(&addr).unwrap();

        socket.incoming().for_each(move |connection| {
            let peer_addr = connection.peer_addr();
            let codec = json::JsonFrameCodec::default();

            let config = swap_config(self.sender.clone());

            let connection = Connection::new(config, codec, connection);
            let (close_future, _client) = connection.start::<json::JsonFrameHandler>();

            tokio::spawn(close_future.then(move |result| {
                match result {
                    Ok(()) => info!("Connection with {:?} closed", peer_addr),
                    Err(e) => error!(
                        "Unexpected error in connection with {:?}: {:?}",
                        peer_addr, e
                    ),
                }
                Ok(())
            }));
            Ok(())
        })
    }
}

#[derive(Debug)]
pub struct ComitServer {
    sender: mpsc::UnboundedSender<(
        SwapId,
        rfc003::bob::SwapRequestKind,
        oneshot::Sender<rfc003::bob::SwapResponseKind>,
    )>,
}
