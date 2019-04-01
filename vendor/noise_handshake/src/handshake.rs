use futures::*;
use snow::Session;
use tokio::io::{AsyncRead, AsyncWrite};

const NOISE_MAX_SIZE: usize = 65535;
// Hack: the snow library panics if trying to decode a buffer smaller than 48
// bytes.
const MIN_HANDSHAKE_SIZE: usize = 48;

#[allow(missing_debug_implementations)]
pub enum Step {
	Read {
		enc_buffer: [u8; NOISE_MAX_SIZE],
		len: usize,
	},
	Write {
		to_write: [u8; NOISE_MAX_SIZE],
		written_bytes: usize,
		total_bytes: usize,
	},
}

impl Step {
	fn read() -> Self {
		Step::Read {
			enc_buffer: [0u8; NOISE_MAX_SIZE],
			len: 0,
		}
	}

	fn write(noise: &mut Session) -> Self {
		let mut buffer = [0u8; NOISE_MAX_SIZE];
		let len = noise
			.write_message(&[], &mut buffer)
			.expect("A zero-length message cannot be too long");
		Step::Write {
			to_write: buffer,
			written_bytes: 0,
			total_bytes: len,
		}
	}
}

pub trait Handshake {
	fn handshake<IO>(self, io: IO) -> NoiseHandshake<IO>
	where
		IO: AsyncRead + AsyncWrite;
}

impl Handshake for Session {
	fn handshake<IO>(mut self, io: IO) -> NoiseHandshake<IO>
	where
		IO: AsyncRead + AsyncWrite,
	{
		match self {
			Session::Handshake(ref handshake_state) => NoiseHandshake {
				next: if handshake_state.is_initiator() {
					Step::write(&mut self)
				} else {
					Step::read()
				},
				noise: Some(self),
				io: Some(io),
			},
			_ => panic!(
				"Noise Session in incorrect state, you should init before starting handshake"
			),
		}
	}
}

#[allow(missing_debug_implementations)]
pub struct NoiseHandshake<IO: AsyncRead + AsyncWrite> {
	next: Step,
	noise: Option<Session>,
	io: Option<IO>,
}

impl<IO: AsyncRead + AsyncWrite> NoiseHandshake<IO> {
	fn wrap_up(&mut self) -> (Session, IO) {
		let noise = self.noise.take().expect("We know it's a Some");
		let noise = noise
			.into_transport_mode()
			.expect("Should not fail as handshake is finished");
		let io = self.io.take().expect("We know it's a Some");
		(noise, io)
	}
}

impl<IO: AsyncRead + AsyncWrite> Future for NoiseHandshake<IO> {
	type Item = (Session, IO);
	type Error = std::io::Error;

	fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
		match self {
			Self {
				noise: Some(ref mut noise),
				io: Some(ref mut io),
				next:
					Step::Write {
						to_write,
						ref mut written_bytes,
						total_bytes,
					},
			} => {
				while written_bytes < total_bytes {
					*written_bytes =
						try_ready!(io.poll_write(&to_write[*written_bytes..*total_bytes]));
				}
				if noise.is_handshake_finished() {
					Ok(Async::Ready(self.wrap_up()))
				} else {
					self.next = Step::read();
					self.poll()
				}
			}
			Self {
				noise: Some(ref mut noise),
				io: Some(ref mut io),
				next: Step::Read {
					mut enc_buffer,
					ref mut len,
				},
			} => {
				let mut dec_buffer = [0u8; NOISE_MAX_SIZE];

				*len += try_ready!(io.poll_read(&mut enc_buffer[*len..]));
				if *len < MIN_HANDSHAKE_SIZE {
					self.poll()
				} else {
					match noise.read_message(&enc_buffer[..*len], &mut dec_buffer) {
						Ok(_) => {
							if noise.is_handshake_finished() {
								Ok(Async::Ready(self.wrap_up()))
							} else {
								self.next = Step::write(noise);
								self.poll()
							}
						}
						Err(_) => self.poll(),
					}
				}
			}
			Self { noise: None, .. } | Self { io: None, .. } => {
				panic!("Future is already resolved!");
			}
		}
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use futures::future::poll_fn;
	use std::thread;
	use tokio::net::{TcpListener, TcpStream};

	fn setup() -> (
		impl Future<Item = (Session, TcpStream), Error = std::io::Error>,
		impl Future<Item = (Session, TcpStream), Error = std::io::Error>,
	) {
		let builder_resp = snow::Builder::new("Noise_XK_25519_ChaChaPoly_BLAKE2s".parse().unwrap());
		let static_keypair_resp = builder_resp.generate_keypair().unwrap();
		let noise_resp = builder_resp
			.local_private_key(&static_keypair_resp.private)
			.build_responder()
			.unwrap();

		let builder_init = snow::Builder::new("Noise_XK_25519_ChaChaPoly_BLAKE2s".parse().unwrap());
		let static_key_init = builder_init.generate_keypair().unwrap().private;
		let noise_init = builder_init
			.local_private_key(&static_key_init)
			.remote_public_key(&static_keypair_resp.public) // The initiator already knows the responder public key
			.build_initiator()
			.unwrap();

		let addr = "127.0.0.1:0".parse().unwrap();

		let mut listener = TcpListener::bind(&addr).expect("unable to bind TCP Listener");
		let addr = listener.local_addr().expect("Did not bind?");

		let listener_future = poll_fn(move || listener.poll_accept());

		let handshake_resp =
			listener_future.and_then(move |(socket, _)| noise_resp.handshake(socket));

		let handshake_init =
			TcpStream::connect(&addr).and_then(move |socket| noise_init.handshake(socket));

		(handshake_init, handshake_resp)
	}

	#[test]
	fn handshake() -> Result<(), std::io::Error> {
		let (hs_init, hs_resp) = setup();

		let (sender_init, receiver_init) = futures::sync::oneshot::channel();

		thread::spawn(move || {
			let mut runtime = tokio::runtime::Runtime::new().unwrap();

			let result = runtime.block_on(hs_init).map(|(session, _)| session);
			sender_init.send(result).unwrap();
		});

		let (sender_resp, receiver_resp) = futures::sync::oneshot::channel();

		thread::spawn(move || {
			let mut runtime = tokio::runtime::Runtime::new().unwrap();

			let result = runtime.block_on(hs_resp).map(|(session, _)| session);
			sender_resp.send(result).unwrap();
		});

		let receivers = receiver_init.join(receiver_resp);

		let mut runtime = tokio::runtime::Runtime::new().unwrap();

		let result = runtime.block_on(receivers).unwrap();

		match result {
			(Ok(Session::Transport(_)), Ok(Session::Transport(_))) => Ok(()),
			err => panic!("Sessions not in expected state: {:?}", err),
		}
	}

}
