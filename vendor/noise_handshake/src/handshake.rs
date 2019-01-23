use futures::*;
use snow::Session::{self, Handshake};
use tokio::io::{AsyncRead, AsyncWrite};

const NOISE_MAX_SIZE: usize = 65535;

#[derive(DebugStub)]
pub enum Step {
    Read {
        #[debug_stub = "encrypted buffer"]
        enc_buffer: [u8; NOISE_MAX_SIZE],
        len: usize,
    },
    Write {
        #[debug_stub = "buffer"]
        buffer: [u8; NOISE_MAX_SIZE],
        remaining_bytes: usize,
        len: usize,
    },
}

impl Step {
    fn read() -> Self {
        Step::Read {
            enc_buffer: [0u8; NOISE_MAX_SIZE],
            len: 0,
        }
    }

    fn write() -> Self {
        Step::Write {
            buffer: [0u8; NOISE_MAX_SIZE],
            remaining_bytes: 0,
            len: 0,
        }
    }
}

pub trait InitHandshake {
    fn init_hand_shake<IO>(self, io: IO) -> NoiseHandshake<IO>
    where
        IO: AsyncRead + AsyncWrite;
}

impl InitHandshake for Session {
    fn init_hand_shake<IO>(self, io: IO) -> NoiseHandshake<IO>
    where
        IO: AsyncRead + AsyncWrite,
    {
        match self {
            Handshake(ref handshake_state) => NoiseHandshake {
                next: if handshake_state.is_initiator() {
                    Step::write()
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

#[derive(Debug)]
pub struct NoiseHandshake<IO: AsyncRead + AsyncWrite> {
    next: Step,
    noise: Option<Session>,
    io: Option<IO>,
}

impl<IO: AsyncRead + AsyncWrite> Future for NoiseHandshake<IO> {
    type Item = (Session, IO);
    // TODO: probably a custom error when removing unwraps
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self {
            Self {
                noise: Some(noise),
                io: Some(_),
                ..
            } if noise.is_handshake_finished() => {
                let noise = self.noise.take().expect("We know it's a Some");
                let noise = noise.into_transport_mode().unwrap();
                let io = self.io.take().expect("We know it's a Some");
                Ok(Async::Ready((noise, io)))
            }
            Self {
                noise: Some(ref mut noise),
                io: Some(ref mut io),
                next:
                    Step::Write {
                        mut buffer,
                        mut remaining_bytes,
                        mut len,
                    },
            } if remaining_bytes == 0 => {
                len = noise.write_message(&[], &mut buffer).unwrap();

                let mut num_bytes_written = 0;
                while num_bytes_written < len {
                    num_bytes_written = try_ready!(io.poll_write(&buffer[num_bytes_written..len]));
                    #[allow(unused_assignments)]
                    // remaining_bytes is a mut from the matching pattern
                    // hence this assignment modify self. If at the next loop try_ready! returns
                    // then the value stored in remaining_bytes will be read 2
                    // lines above
                    remaining_bytes = len - num_bytes_written;
                }
                self.next = Step::read();
                self.poll()
            }
            Self {
                noise: Some(ref mut _noise),
                io: Some(ref mut io),
                next:
                    Step::Write {
                        buffer,
                        mut remaining_bytes,
                        len,
                    },
                ..
            } => {
                let mut num_bytes_written = *len - remaining_bytes;
                while num_bytes_written < *len {
                    num_bytes_written = try_ready!(io.poll_write(&buffer[num_bytes_written..*len]));
                    #[allow(unused_assignments)]
                    // remaining_bytes is a mut from the matching pattern
                    // hence this assignment modify self. If at the next loop try_ready! returns
                    // then the value stored in remaining_bytes will be read 2
                    // lines above
                    remaining_bytes = *len - num_bytes_written;
                }
                self.next = Step::read();
                self.poll()
            }
            Self {
                noise: Some(ref mut noise),
                io: Some(ref mut io),
                next:
                    Step::Read {
                        mut enc_buffer,
                        mut len,
                    },
            } => {
                let mut dec_buffer = [0u8; NOISE_MAX_SIZE];

                loop {
                    len = try_ready!(io.poll_read(&mut enc_buffer[len..]));
                    match noise.read_message(&enc_buffer[..len], &mut dec_buffer) {
                        Ok(_) => break,
                        Err(e) => debug!("Error decoding message: {:?}", e),
                    };
                }
                self.next = Step::write();
                self.poll()
            }
            Self { noise: None, .. } => {
                panic!("Future is already resolved!");
            }
            Self { io: None, .. } => {
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

        let handshake_init = noise_init.init_hand_shake(socket_init);

        let handshake_resp = noise_resp.init_hand_shake(socket_resp);

        (handshake_init, handshake_resp)
    }

    #[test]
    fn handshake() -> Result<(), std::io::Error> {
        let (hs_init, hs_resp) = setup();

        let mut runtime = tokio::runtime::Runtime::new()?;

        let hs_init = hs_init.map_err(|_| ());
        //        let hs_resp = hs_resp.map_err(|_| ());

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
