use comit_client::{client::Client, DefaultClient, FakeClient};
use std::{io, marker::PhantomData, net::SocketAddr};

#[derive(Debug)]
pub enum FactoryError {
    Connection(io::Error),
}

pub trait Factory<C: Client>: Send + Sync {
    fn client_for(&self, comit_node_socket_addr: SocketAddr) -> Result<C, FactoryError>;
}

pub struct DefaultFactory {}

impl DefaultFactory {
    pub fn new() -> Self {
        Self {}
    }
}

impl Factory<DefaultClient> for DefaultFactory {
    fn client_for(
        &self,
        comit_node_socket_addr: SocketAddr,
    ) -> Result<DefaultClient, FactoryError> {
        unimplemented!()
    }
}

pub struct FakeFactory<F> {
    phantom: PhantomData<F>,
}

impl<C: Default + Client + Send + Sync> Factory<C> for FakeFactory<C> {
    fn client_for(&self, comit_node_socket_addr: SocketAddr) -> Result<C, FactoryError> {
        Ok(C::default())
    }
}

// let (mut client, _shutdown_handle) = self
//     .connect_to_comit_node(&mut runtime)
//     .map_err(SwapRequestError::FailedToConnect)?;

// fn connect_to_comit_node(
//     &self,
//     runtime: &mut Runtime,
// ) -> (Result<
//     (
//         Client<json::Frame, json::Request, json::Response>,
//         ShutdownHandle,
//     ),
//     io::Error,
// >) {
//     info!(
//         "Connecting to {} to make request",
//         &self.comit_node_socket_addr
//     );
//     let socket = TcpStream::connect(&self.comit_node_socket_addr).wait()?;
//     let codec = json::JsonFrameCodec::default();
//     let config: Config<json::Request, json::Response> = Config::new();
//     let connection = Connection::new(config, codec, socket);

//     let (connection_future, client) = connection.start::<json::JsonFrameHandler>();
//     let (connection_future, shutdown_handle) = shutdown_handle::new(connection_future);
//     let socket_addr = self.comit_node_socket_addr.clone();

//     runtime.spawn(connection_future.map_err(move |e| {
//         error!(
//             "Connection to {:?} prematurely closed: {:?}",
//             socket_addr, e
//         )
//     }));

//     Ok((client, shutdown_handle))
// }
