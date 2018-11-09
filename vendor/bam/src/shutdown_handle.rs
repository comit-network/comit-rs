use futures::{
    sync::oneshot::{self, Sender},
    Future,
};

#[derive(Debug)]
pub struct ShutdownHandle {
    sender: Option<Sender<()>>,
}

impl Drop for ShutdownHandle {
    fn drop(&mut self) {
        let sender = self.sender.take().unwrap();
        if let Ok(()) = sender.send(()) {
            debug!("Shut down server because handle is dropped.");
        }
    }
}

pub fn new<E>(
    future: impl Future<Item = (), Error = E>,
) -> (impl Future<Item = (), Error = E>, ShutdownHandle) {
    let (sender, receiver) = oneshot::channel();

    let combined_future = future
        .select(receiver.map_err(|_| unreachable!()))
        .and_then(|_| Ok(()))
        .map_err(|(e, _)| e);

    let _self = ShutdownHandle {
        sender: Some(sender),
    };

    (combined_future, _self)
}
