use bitcoin_support::{deserialize, Block};
use futures::sync::mpsc::{self, UnboundedReceiver};
use std::thread;
use zmq_rs::{self as zmq, Context, Socket};

pub fn bitcoin_block_listener(endpoint: &str) -> Result<UnboundedReceiver<Block>, zmq::Error> {
    let context = Context::new()?;
    let mut socket = context.socket(zmq::SUB)?;

    socket.set_subscribe(b"rawblock")?;
    socket.connect(endpoint)?;

    log::info!(
        "Connecting to {} to subscribe to new Bitcoin blocks over ZeroMQ",
        socket.get_last_endpoint().unwrap()
    );

    let (state_sender, state_receiver) = mpsc::unbounded();

    thread::spawn(move || {
        // we need this to keep the context alive
        let _context = context;

        loop {
            let result = receive_block(&mut socket);

            if let Ok(Some(block)) = result {
                let _ = state_sender.unbounded_send(block);
            }
        }
    });
    Ok(state_receiver)
}

fn receive_block(socket: &mut Socket) -> Result<Option<Block>, zmq::Error> {
    let bytes = socket.recv_bytes(zmq::SNDMORE)?;
    let bytes: &[u8] = bytes.as_ref();

    match bytes {
        b"rawblock" => {
            let bytes = socket.recv_bytes(zmq::SNDMORE)?;

            match deserialize(bytes.as_ref()) {
                Ok(block) => {
                    log::trace!("Got {:?}", block);
                    Ok(Some(block))
                }
                Err(e) => {
                    log::error!("Got new block but failed to deserialize it because {:?}", e);
                    Ok(None)
                }
            }
        }
        _ => {
            log::error!("Unhandled message: {:?}", bytes);
            Ok(None)
        }
    }
}
