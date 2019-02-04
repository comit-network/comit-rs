use futures::*;
use snow::Session;
use tokio::io::{AsyncRead, AsyncWrite};

const NOISE_MAX_SIZE: usize = 65535;

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
                ..
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
                next:
                    Step::Read {
                        mut enc_buffer,
                        ref mut len,
                    },
            } => {
                let mut dec_buffer = [0u8; NOISE_MAX_SIZE];

                *len += try_ready!(io.poll_read(&mut enc_buffer[*len..]));

                match noise.read_message(&enc_buffer[..*len], &mut dec_buffer) {
                    Ok(_) => {
                        if !noise.is_handshake_finished() {
                            self.next = Step::write(noise);
                            self.poll()
                        } else {
                            Ok(Async::Ready(self.wrap_up()))
                        }
                    }
                    Err(_e) => {
                        trace!(
                            "Re-polling because a single poll_read didn't have the whole message"
                        );
                        self.poll()
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

    fn setup() -> (
        NoiseHandshake<memsocket::UnboundedSocket>,
        NoiseHandshake<memsocket::UnboundedSocket>,
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

        let (socket_init, socket_resp) = memsocket::unbounded();

        let handshake_init = noise_init.handshake(socket_init);

        let handshake_resp = noise_resp.handshake(socket_resp);

        (handshake_init, handshake_resp)
    }

    #[test]
    fn handshake() -> Result<(), std::io::Error> {
        let (hs_init, hs_resp) = setup();

        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let hs_init = hs_init.map_err(|_| ());

        runtime.spawn(hs_init.and_then(|(noise, _io)| match noise {
            Session::Transport(_) => Ok(()),
            _ => panic!("Initiator session is expected to be in transport mode"),
        }));

        let result = runtime
            .block_on_all(hs_resp.and_then(|(noise, _io)| match noise {
                Session::Transport(_) => Ok(true),
                _ => Ok(false),
            }))
            .unwrap();

        assert!(result);

        Ok(())
    }

}
