use crate::{
	bam_api::rfc003::swap_config, node_id::NodeId, swap_protocols::rfc003::bob::BobSpawner,
};
use bam::{self, json};
use futures::{
	future::{self, Shared},
	Future,
};
use std::{
	collections::HashMap,
	io,
	net::SocketAddr,
	sync::{Arc, Mutex, RwLock},
};
use tokio::{
	self,
	io::{AsyncRead, AsyncWrite},
	net::TcpStream,
};

type BamClient =
	Arc<Mutex<bam::client::Client<json::Frame, json::OutgoingRequest, json::Response>>>;

type WaitList = Shared<Box<dyn Future<Item = BamClient, Error = io::ErrorKind> + Send>>;

#[derive(Derivative)]
#[derivative(Debug)]
enum ConnectionState {
	Connected {
		#[derivative(Debug = "ignore")]
		client: BamClient,
	},
	Connecting {
		#[derivative(Debug = "ignore")]
		waitlist: WaitList,
	},
	Disconnected,
}

/// Keeps track of connections per NodeId.
/// For now we don't allow multiple connections from the same NodeId.
#[derive(Default, Debug)]
pub struct ConnectionPool {
	connections: RwLock<HashMap<NodeId, Arc<Mutex<ConnectionState>>>>,
}

impl ConnectionState {
	fn transition_to_connecting(&mut self, waitlist: &WaitList) {
		*self = ConnectionState::Connecting {
			waitlist: waitlist.clone(),
		};
	}

	fn transition_to_connected(&mut self, client: &BamClient) {
		*self = ConnectionState::Connected {
			client: Arc::clone(client),
		}
	}

	fn transition_to_disconnected(&mut self) {
		*self = ConnectionState::Disconnected;
	}
}

impl ConnectionPool {
	pub fn client_for<B: BobSpawner>(
		&self,
		node_id: NodeId,
		bob_spawner: B,
	) -> Box<dyn Future<Item = BamClient, Error = io::ErrorKind> + Send> {
		debug!("Trying to get client for {}", node_id);
		let mut connections = self.connections.write().unwrap();
		let existing_connection = connections
			.entry(node_id)
			.or_insert_with(|| Arc::new(Mutex::new(ConnectionState::Disconnected)));

		let mut connection_state = (*existing_connection).lock().unwrap();

		match *connection_state {
			ConnectionState::Disconnected => {
				info!("No existing connection to {}. Trying to connect.", node_id);
				let client_future = Self::make_new_connection(
					node_id,
					Arc::clone(&existing_connection),
					bob_spawner,
				);

				let waitlist = client_future.shared();

				connection_state.transition_to_connecting(&waitlist);

				Box::new(Self::add_to_waitlist(&waitlist))
			}
			ConnectionState::Connecting { ref waitlist } => {
				debug!(
					"Already in the process of connecting to {}. Joining the waitlist.",
					node_id
				);
				Box::new(Self::add_to_waitlist(waitlist))
			}
			ConnectionState::Connected { ref client } => {
				debug!("Retrieved existing client for {}", node_id);
				Box::new(future::ok(Arc::clone(&client)))
			}
		}
	}

	fn add_to_waitlist(
		waitlist: &WaitList,
	) -> impl Future<Item = BamClient, Error = io::ErrorKind> + Send {
		waitlist.clone().then(|result| match result {
			Ok(client) => Ok((*client).clone()),
			Err(e) => Err(*e),
		})
	}

	fn make_new_connection<B: BobSpawner>(
		node_id: NodeId,
		connection_handle: Arc<Mutex<ConnectionState>>,
		bob_spawner: B,
	) -> Box<dyn Future<Item = BamClient, Error = io::ErrorKind> + Send> {
		Box::new(TcpStream::connect(&node_id).then(move |socket| {
			Self::new_outgoing_socket(node_id, socket, connection_handle, bob_spawner)
				.map_err(|e| e.kind())
		}))
	}

	/// When we get a socket for someone we were trying to connect to
	fn new_outgoing_socket<B: BobSpawner>(
		node_id: NodeId,
		socket: Result<TcpStream, io::Error>,
		connection_handle: Arc<Mutex<ConnectionState>>,
		bob_spawner: B,
	) -> Result<BamClient, io::Error> {
		let mut connection_state = connection_handle.lock().unwrap();
		match *connection_state {
            // The most usual case: We were in Connecting and now we have a socket
            // we want to transition to Connected
            ConnectionState::Connecting { .. } |
            // Somehow we got an outgoing connection when we're in the Disconnected (rather than Connecting) state.
            // This shouldn't happen, but just set up the connection anyway.
            ConnectionState::Disconnected => match socket {
                Ok(socket) => {
                    info!("Successfully connected to {} while in the {:?} state", node_id, connection_state);
                    let client = Self::spawn_new_connection(
                        node_id,
                        Arc::clone(&connection_handle),
                        bob_spawner,
                        socket,
                    );

                    connection_state.transition_to_connected(&client);
                    Ok(client)
                }
                Err(e) => {
                    error!("Failed to connect to {}: {:?}", node_id, e);
                    connection_state.transition_to_disconnected();
                    Err(e)
                }
            },
            ConnectionState::Connected { ref client } => {
                // We're already connected by the time we managed to connect.
                // Forget about this new one and just use the old one.
                match socket {
                    Ok(_socket) => {
                        debug!("Successfully connected to {} but we already have an exsting connection", node_id);
                        // let the socket go out of scope and get dropped
                        Ok(Arc::clone(&client))
                    }
                    Err(e) => {
                        error!("Failed to connect to {} (but we already have an existing connection): {:?} ",node_id, e);
                        Ok(Arc::clone(&client))
                    }
                }
            }
        }
	}

	/// Processes a new server socket
	pub fn new_incoming_socket<B: BobSpawner>(&self, socket: TcpStream, bob_spawner: B) {
		let node_id = match socket.peer_addr() {
			Ok(node_id) => node_id,
			Err(e) => {
				error!("Couldn't get peer address: {:?}", e);
				return;
			}
		};
		let mut connections = self.connections.write().unwrap();

		let existing_connection = connections
			.entry(node_id)
			.or_insert_with(|| Arc::new(Mutex::new(ConnectionState::Disconnected)));

		let mut connection_state = (*existing_connection).lock().unwrap();

		match *connection_state {
			ConnectionState::Disconnected => {
				let client = Self::spawn_new_connection(
					node_id,
					Arc::clone(&existing_connection),
					bob_spawner,
					socket,
				);

				connection_state.transition_to_connected(&client);
			}
			ConnectionState::Connected { .. } => {
				warn!("Ignoring incoming connection from {} because we already have a connection to them", node_id);
			}
			ConnectionState::Connecting { .. } => {
				warn!(
					"Ignoring incoming connection from {} because we're already connecting",
					node_id
				);
			}
		}
	}

	/// Creates a connection from the socket and spawns it.
	fn spawn_new_connection<S: AsyncRead + AsyncWrite + Send + 'static, B: BobSpawner>(
		node_id: NodeId,
		connection_handle: Arc<Mutex<ConnectionState>>,
		bob_spawner: B,
		socket: S,
	) -> BamClient {
		info!("Established new connection to {}", node_id);

		let (client, connection) = Self::set_up_bam_connection(socket, swap_config(bob_spawner));

		let final_connection_state = Arc::clone(&connection_handle);

		tokio::spawn(connection.then(move |result| {
			let mut final_connection_state = final_connection_state.lock().unwrap();

			match result {
				Ok(_) => {
					info!(
						"Connection to {} was closed while it was in the {:?} state",
						node_id, final_connection_state
					);
				}
				Err(e) => {
					error!(
						"Connection to {} prematurely closed while it was in the {:?} state: {:?}",
						node_id, final_connection_state, e
					);
				}
			}
			final_connection_state.transition_to_disconnected();

			Ok(())
		}));

		Arc::new(Mutex::new(client))
	}

	fn set_up_bam_connection<S: AsyncRead + AsyncWrite + Send + 'static>(
		socket: S,
		config: bam::config::Config<json::ValidatedIncomingRequest, json::Response>,
	) -> (
		bam::client::Client<json::Frame, json::OutgoingRequest, json::Response>,
		impl Future<Item = (), Error = bam::connection::ClosedReason<json::Error>> + Send,
	) {
		let codec = json::JsonFrameCodec::default();

		let response_source = Arc::new(Mutex::new(json::JsonResponseSource::default()));
		let incoming_frames = json::JsonFrameHandler::create(config, Arc::clone(&response_source));
		let (client, outgoing_frames) = bam::client::Client::create(response_source);

		(
			client,
			bam::connection::new(codec, socket, incoming_frames, outgoing_frames),
		)
	}

	pub fn connected_addrs(&self) -> Vec<SocketAddr> {
		let clients = self.connections.read().unwrap();

		let mut keys = Vec::new();
		for key in clients.keys() {
			keys.push(key.clone());
		}
		keys
	}
}
