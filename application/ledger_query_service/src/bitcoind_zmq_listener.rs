use crate::{
    block_processor::BlockProcessor,
    zmq::{self, Context, Socket},
};
use bitcoin_support::{serialize::deserialize, MinedBlock};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

#[derive(DebugStub)]
pub struct BitcoindZmqListener<P> {
    #[debug_stub = "Context"]
    _context: Context,
    #[debug_stub = "Socket"]
    socket: Socket,
    #[debug_stub = "Processor"]
    processor: P,
}

impl<P: BlockProcessor<MinedBlock>> BitcoindZmqListener<P> {
    pub fn new(endpoint: &str, processor: P) -> Result<Self, zmq::Error> {
        let context = Context::new()?;
        let mut socket = context.socket(zmq::SUB)?;

        socket.set_subscribe(b"rawblock")?;
        socket.connect(endpoint)?;

        Ok(BitcoindZmqListener {
            _context: context,
            socket,
            processor,
        })
    }

    pub fn start(&mut self) {
        info!(
            "Connecting to {} to subscribe to new Bitcoin blocks over ZeroMQ",
            self.socket.get_last_endpoint().unwrap()
        );

        loop {
            let result = self.receive_block();

            if let Ok(Some(block)) = result {
                self.processor.process(&block);
            }
        }
    }

    fn receive_block(&mut self) -> Result<Option<MinedBlock>, zmq::Error> {
        let bytes = self.socket.recv_bytes(zmq::SNDMORE)?;
        let bytes: &[u8] = bytes.as_ref();

        match bytes {
            b"rawblock" => {
                let bytes = self.socket.recv_bytes(zmq::SNDMORE)?;
                let block_height = self.socket.recv_bytes(zmq::SNDMORE)?;

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
                debug!("Unhandled message: {:?}", bytes);
                Ok(None)
            }
        }
    }
}
