use crate::swap_protocols::{
    metadata_store::MetadataStore, rfc003::state_store::StateStore, ProtocolDependencies, SwapId,
};
use futures::{Future, Stream};
use std::{io, net::SocketAddr};
use tokio::{self, net::TcpListener};

pub fn listen<T: MetadataStore<SwapId>, S: StateStore>(
    addr: SocketAddr,
    protocol_dependencies: ProtocolDependencies<T, S>,
) -> impl Future<Item = (), Error = io::Error> {
    info!("ComitServer listening at {:?}", addr);
    let socket = TcpListener::bind(&addr).unwrap();

    socket.incoming().for_each(move |socket| {
        protocol_dependencies
            .connection_pool
            .new_incoming_socket(socket, protocol_dependencies.clone());
        Ok(())
    })
}
