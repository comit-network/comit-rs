use bitcoin_support::{serialize::deserialize, Block, Transaction};
use transaction_processor::TransactionProcessor;
use zmq::{self, Context, Socket};

pub struct BitcoinZmqListener<P> {
    context: Context,
    socket: Socket,
    processor: P,
}

impl<P: TransactionProcessor<Transaction>> BitcoinZmqListener<P> {
    pub fn new(endpoint: &str, processor: P) -> Self {
        let context = Context::new().unwrap();
        let mut socket = context.socket(zmq::SUB).unwrap();

        socket.set_subscribe(b"rawblock").unwrap();
        socket.connect(endpoint).unwrap();

        BitcoinZmqListener {
            context,
            socket,
            processor,
        }
    }

    pub fn start(&mut self) {
        loop {
            let result = self.receive_msg();

            if let Ok(Some(block)) = result {
                block
                    .txdata
                    .iter()
                    .for_each(|tx| self.processor.process(tx))
            }
        }
    }

    fn receive_msg(&mut self) -> Result<Option<Block>, zmq::Error> {
        let bytes = self.socket.recv_bytes(zmq::SNDMORE)?;
        let bytes: &[u8] = bytes.as_ref();

        match bytes {
            b"rawblock" => {
                let bytes = self.socket.recv_bytes(zmq::SNDMORE)?;

                match deserialize(bytes.as_ref()) {
                    Ok(block) => {
                        info!("Got new block {:?}", block);
                        Ok(Some(block))
                    }
                    Err(e) => {
                        error!("Got new block but failed to deserialize {:?}", e);
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
