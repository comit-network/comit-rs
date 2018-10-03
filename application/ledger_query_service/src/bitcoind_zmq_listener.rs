use bitcoin_support::{serialize::deserialize, Block};
use block_processor::BlockProcessor;
use zmq::{self, Context, Socket};

#[derive(DebugStub)]
pub struct BitcoindZmqListener<P> {
    #[debug_stub = "Context"]
    _context: Context,
    #[debug_stub = "Socket"]
    socket: Socket,
    #[debug_stub = "Processor"]
    processor: P,
}

impl<P: BlockProcessor<Block>> BitcoindZmqListener<P> {
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

        // for now work on the assumption that there is one blockchain, but warn
        // every time that assumption doesn't hold, by comparing the previous
        // blockhash to the most recent member of a list of ordered blockhashes
        loop {
            let result = self.receive_block();

            if let Ok(Some(block)) = result {
                self.processor.process(&block);
            }
        }
    }

    fn receive_block(&mut self) -> Result<Option<Block>, zmq::Error> {
        let bytes = self.socket.recv_bytes(zmq::SNDMORE)?;
        let bytes: &[u8] = bytes.as_ref();

        match bytes {
            b"rawblock" => {
                let bytes = self.socket.recv_bytes(zmq::SNDMORE)?;

                match deserialize(bytes.as_ref()) {
                    Ok(block) => {
                        trace!("Got {:?}", block);
                        Ok(Some(block))
                    }
                    Err(e) => {
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
