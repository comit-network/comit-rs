use crate::{
    bam_api::rfc003::swap_config,
    comit_client::{
        bam::{BamClient, BamClientPool},
        ClientFactory,
    },
    swap_protocols::rfc003::bob::BobSpawner,
};
use bam::{self, connection, json};
use futures::{Future, Stream};
use std::{io, net::SocketAddr, sync::Arc};
use tokio::{self, net::TcpListener};

pub fn listen<B: BobSpawner>(
    addr: SocketAddr,
    bob_spawner: Arc<B>,
    bam_client_pool: Arc<BamClientPool>,
) -> impl Future<Item = (), Error = io::Error> {
    info!("ComitServer listening at {:?}", addr);
    let socket = TcpListener::bind(&addr).unwrap();

    socket.incoming().for_each(move |connection| {
        let peer_addr = connection.peer_addr();
        let codec = json::JsonFrameCodec::default();

        let (incoming_frames, response_source) =
            json::JsonFrameHandler::create(swap_config(Arc::clone(&bob_spawner)));
        let (client, outgoing_frames) = bam::client::Client::create(response_source);

        let connection = connection::new(codec, connection, incoming_frames, outgoing_frames);

        if let Ok(addr) = peer_addr {
            let bam_client = Arc::new(BamClient::new(addr, client));
            bam_client_pool.add_client(addr, bam_client);
        }

        tokio::spawn(connection.then(move |result| {
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
