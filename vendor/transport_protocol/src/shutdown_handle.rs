use futures::{
    sync::oneshot::{self, Sender},
    Future,
};

pub struct ShutdownHandle {
    sender: Option<Sender<()>>,
}

impl Drop for ShutdownHandle {
    fn drop(&mut self) {
        debug!("Shutting down server because handle is dropped.");

        let sender = self.sender.take().unwrap();
        sender.send(()).unwrap()
    }
}

pub fn new(
    future: impl Future<Item = (), Error = ()>,
) -> (impl Future<Item = (), Error = ()>, ShutdownHandle) {
    let (sender, receiver) = oneshot::channel();

    let combined_future = future.select(receiver.map_err(|_| ())).then(|_| Ok(()));

    let _self = ShutdownHandle {
        sender: Some(sender),
    };

    (combined_future, _self)
}
