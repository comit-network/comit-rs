use bitcoin_support::{
    serialize::{deserialize, BitcoinHash},
    Block, Sha256dHash, Transaction,
};
use transaction_processor::TransactionProcessor;
use zmq::{self, Context, Socket};

#[derive(DebugStub)]
pub struct BitcoindZmqListener<P> {
    #[debug_stub = "Context"]
    _context: Context,
    #[debug_stub = "Socket"]
    socket: Socket,
    #[debug_stub = "Processor"]
    processor: P,
    blockhashes: Vec<Sha256dHash>,
}

impl<P: TransactionProcessor<Transaction>> BitcoindZmqListener<P> {
    pub fn new(endpoint: &str, processor: P) -> Result<Self, zmq::Error> {
        let context = Context::new()?;
        let mut socket = context.socket(zmq::SUB)?;

        socket.set_subscribe(b"rawblock")?;
        socket.connect(endpoint)?;

        Ok(BitcoindZmqListener {
            _context: context,
            socket,
            processor,
            blockhashes: Vec::new(),
        })
    }

    pub fn start(&mut self) {
        info!(
            "Connecting to {} to subscribe to new Bitcoin blocks over ZeroMQ",
            self.socket.get_last_endpoint().unwrap()
        );

        // for now work on the assumption that there is one blockchain, but warn
        // every time that assumption doesn't hold, by comparing prev_blockhash to
        // the most recent member of a list of ordered blockhashes, obtained using
        // the method bitcoin_hash
        loop {
            let result = self.receive_block();

            if let Ok(Some(block)) = result {
                match self.blockhashes.last() {
                    Some(last_blockhash) => {
                        if *last_blockhash != block.header.prev_blockhash {
                            warn!(
                                "Last blockhash in chain doesn't match with block {} previous blockhash",
                                block.header.bitcoin_hash()
                            );
                        }
                    }
                    None => (),
                }
                self.blockhashes.push(block.header.bitcoin_hash());

                self.processor.update_unconfirmed_txs_queue();

                block
                    .txdata
                    .iter()
                    .for_each(|tx| self.processor.process(tx))
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
