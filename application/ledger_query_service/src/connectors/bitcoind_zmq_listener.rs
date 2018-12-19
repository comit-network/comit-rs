use bitcoin_support::{serialize::deserialize, MinedBlock};
use byteorder::{LittleEndian, ReadBytesExt};
use futures::sync::mpsc::{self, UnboundedReceiver};
use std::{io::Cursor, thread};
use zmq_rs::{self as zmq, Context, Socket};

#[derive(DebugStub)]
pub struct BitcoindZmqListener {}

impl BitcoindZmqListener {
    pub fn create(endpoint: &str) -> Result<UnboundedReceiver<MinedBlock>, zmq::Error> {
        let context = Context::new()?;
        let mut socket = context.socket(zmq::SUB)?;

        socket.set_subscribe(b"rawblock")?;
        socket.connect(endpoint)?;

        info!(
            "Connecting to {} to subscribe to new Bitcoin blocks over ZeroMQ",
            socket.get_last_endpoint().unwrap()
        );

        let (state_sender, state_receiver) = mpsc::unbounded();

        thread::spawn(move || {
            // we need this to keep the context alive
            let _context = context;

            loop {
                let result = Self::receive_block(&mut socket);

                if let Ok(Some(block)) = result {
                    let _ = state_sender.unbounded_send(block);
                }
            }
        });
        Ok(state_receiver)
    }

    fn receive_block(socket: &mut Socket) -> Result<Option<MinedBlock>, zmq::Error> {
        let bytes = socket.recv_bytes(zmq::SNDMORE)?;
        let bytes: &[u8] = bytes.as_ref();

        match bytes {
            b"rawblock" => {
                let bytes = socket.recv_bytes(zmq::SNDMORE)?;
                let block_height = socket.recv_bytes(zmq::SNDMORE)?;

                let mut block_height = Cursor::new(block_height);
                let block_height = block_height.read_u32::<LittleEndian>();

                match (deserialize(bytes.as_ref()), block_height) {
                    (Ok(block), Ok(height)) => {
                        trace!("Got {:?}", block);
                        Ok(Some(MinedBlock::new(block, height)))
                    }
                    (Ok(_), Err(e)) => {
                        error!(
                            "Got new block but failed to extract the height because {:?}",
                            e
                        );
                        Ok(None)
                    }

                    (Err(e), _) => {
                        error!("Got new block but failed to deserialize it because {:?}", e);
                        Ok(None)
                    }
                }
            }
            _ => {
                error!("Unhandled message: {:?}", bytes);
                Ok(None)
            }
        }
    }
}
