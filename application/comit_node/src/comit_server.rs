use crate::{
    bam_api::rfc003::{bob_spawner::BobSpawner, swap_config},
    swap_protocols::{metadata_store::MetadataStore, rfc003::state_store::StateStore, SwapId},
};
use bam::{connection::Connection, json};
use futures::{Future, Stream};
use std::{io, net::SocketAddr, sync::Arc};
use tokio::{self, net::TcpListener};

pub fn listen<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    addr: SocketAddr,
    bob_spawner: Arc<BobSpawner<T, S>>,
) -> impl Future<Item = (), Error = io::Error> {
    info!("ComitServer listening at {:?}", addr);
    let socket = TcpListener::bind(&addr).unwrap();

    socket.incoming().for_each(move |connection| {
        let peer_addr = connection.peer_addr();
        let codec = json::JsonFrameCodec::default();

        let config = swap_config(Arc::clone(&bob_spawner));

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
