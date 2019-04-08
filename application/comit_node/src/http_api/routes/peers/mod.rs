use crate::connection_pool::ConnectionPool;
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc};
use warp::{Rejection, Reply};

#[derive(Debug, Serialize)]
struct GetPeers {
    pub peers: Vec<SocketAddr>,
}

pub fn get_peers(connection_pool: Arc<ConnectionPool>) -> Result<impl Reply, Rejection> {
    let response = GetPeers {
        peers: connection_pool.connected_addrs(),
    };

    Ok(warp::reply::json(&response))
}
