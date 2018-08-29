use futures::{Future, Stream};
use memsocket::{self, UnboundedSocket};
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf},
    runtime::Runtime,
};
use tokio_codec::{FramedRead, LinesCodec};
use transport_protocol::{
    client::Client,
    config::Config,
    connection::Connection,
    json::{Frame, JsonFrameCodec, JsonFrameHandler, Request, Response},
    shutdown_handle::{self, ShutdownHandle},
};

pub struct Alice {
    read: Arc<Mutex<FramedRead<ReadHalf<UnboundedSocket>, LinesCodec>>>,
    write: Arc<Mutex<WriteHalf<UnboundedSocket>>>,
}

impl Alice {
    pub fn send_with_newline(
        &self,
        msg: &'static str,
    ) -> impl Future<Item = (), Error = ::std::io::Error> {
        let write = self.write.clone();

        let send_future = ::futures::future::poll_fn(move || {
            let mut write = write.lock().unwrap();

            let msg = format!("{}\n", msg);

            write.poll_write(msg.as_str().as_bytes())
        });

        send_future.map(|_| ())
    }

    pub fn receive(&self) -> impl Future<Item = Option<String>, Error = ::std::io::Error> {
        let read = self.read.clone();

        ::futures::future::poll_fn(move || {
            let mut read = read.lock().unwrap();

            read.poll()
        })
    }

    #[allow(dead_code)]
    pub fn send_without_newline(
        &self,
        msg: &'static str,
    ) -> impl Future<Item = Option<String>, Error = ::std::io::Error> {
        let write = self.write.clone();
        let read = self.read.clone();

        let send_future = ::futures::future::poll_fn(move || {
            let mut write = write.lock().unwrap();

            write.poll_write(msg.as_bytes())
        });

        let receive_future = ::futures::future::poll_fn(move || {
            let mut read = read.lock().unwrap();

            read.poll()
        });

        send_future.and_then(move |s| {
            debug_assert_eq!(s, msg.len(), "Did not send all bytes!");

            receive_future
        })
    }
}

pub struct Bob {
    _alice: Client<Frame, Request, Response>,
    _shutdown_handle: ShutdownHandle,
}

pub fn create(config: Config<Request, Response>) -> (Runtime, Alice, Bob) {
    let mut runtime = Runtime::new().unwrap();

    let (alice, bob) = memsocket::unbounded();

    let (bob_server, alice_client) =
        Connection::new(config, JsonFrameCodec::default(), bob).start::<JsonFrameHandler>();
    let (bob_server, bob_shutdown_handle) = shutdown_handle::new(bob_server);

    runtime.spawn(bob_server);

    let (read, write) = alice.split();

    let alice = Alice {
        read: Arc::new(Mutex::new(FramedRead::new(read, LinesCodec::new()))),
        write: Arc::new(Mutex::new(write)),
    };

    let bob = Bob {
        _alice: alice_client,
        _shutdown_handle: bob_shutdown_handle,
    };

    (runtime, alice, bob)
}
