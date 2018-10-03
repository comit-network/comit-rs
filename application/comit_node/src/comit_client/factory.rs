use comit_client::{client::Client, DefaultClient, FakeClient};
use futures::Future;
use std::{
    collections::HashMap,
    fmt::Debug,
    io,
    net::SocketAddr,
    panic::RefUnwindSafe,
    sync::{Arc, RwLock},
};
use tokio::{self, net::TcpStream};
use transport_protocol::{config::Config, connection::Connection, json};

#[derive(Debug)]
pub enum FactoryError {
    Connection(io::Error),
}

impl From<io::Error> for FactoryError {
    fn from(e: io::Error) -> Self {
        FactoryError::Connection(e)
    }
}

pub trait Factory<C: Client>: Send + Sync + RefUnwindSafe + Debug {
    fn client_for(&self, comit_node_socket_addr: SocketAddr) -> Result<Arc<C>, FactoryError>;
}

#[derive(Default, Debug)]
pub struct DefaultFactory {
    clients: RwLock<HashMap<SocketAddr, Arc<DefaultClient>>>,
}

impl Factory<DefaultClient> for DefaultFactory {
    fn client_for(
        &self,
        comit_node_socket_addr: SocketAddr,
        //TODO: Return a future and ensure no duplicate connections
    ) -> Result<Arc<DefaultClient>, FactoryError> {
        debug!("Trying to get client for {}", comit_node_socket_addr);
        let existing_client = self
            .clients
            .read()
            .unwrap()
            .get(&comit_node_socket_addr)
            .cloned();

        match existing_client {
            None => {
                info!(
                    "No existing connection to {}. Trying to connect.",
                    comit_node_socket_addr
                );
                let socket = TcpStream::connect(&comit_node_socket_addr).wait()?;
                info!("Connection to {} established", comit_node_socket_addr);
                let codec = json::JsonFrameCodec::default();
                let config = Config::<json::Request, json::Response>::default();
                let connection = Connection::new(config, codec, socket);
                let (connection_future, client) = connection.start::<json::JsonFrameHandler>();
                let socket_addr = comit_node_socket_addr;
                tokio::spawn(connection_future.map_err(move |e| {
                    error!(
                        "Connection to {:?} prematurely closed: {:?}",
                        socket_addr, e
                    )
                }));
                let client = Arc::new(DefaultClient::new(comit_node_socket_addr, client));
                let mut clients = self.clients.write().unwrap();
                clients.insert(comit_node_socket_addr, client.clone());
                debug!(
                    "Client for {} created by making a new connection",
                    comit_node_socket_addr
                );
                Ok(client)
            }
            Some(client) => {
                debug!("Retrieved existing client for {}", comit_node_socket_addr);
                Ok(client.clone())
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct FakeFactory {
    pub fake_client: Arc<FakeClient>,
}

impl FakeFactory {
    pub fn fake_client(&self) -> &FakeClient {
        &self.fake_client
    }
}

impl Factory<FakeClient> for FakeFactory {
    fn client_for(
        &self,
        _comit_node_socket_addr: SocketAddr,
    ) -> Result<Arc<FakeClient>, FactoryError> {
        Ok(self.fake_client.clone())
    }
}
