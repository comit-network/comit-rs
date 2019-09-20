#![allow(missing_debug_implementations)] // Combinators don't need to implement debug

use futures::{Async, Future, Poll, Stream};

pub trait StreamExt: Stream {
    /// Returns a future that resolves with the first element of the stream
    fn first_or_else<F: FnOnce() -> Self::Error>(self, or_else: F) -> FirstOrElse<Self, F>
    where
        Self: Sized,
    {
        FirstOrElse {
            stream: self,
            or_else: Some(or_else),
        }
    }
}

impl<S> StreamExt for S where S: Stream {}

pub struct FirstOrElse<S, F> {
    stream: S,
    or_else: Option<F>,
}

impl<S, I, E, F> Future for FirstOrElse<S, F>
where
    S: Stream<Item = I, Error = E>,
    F: FnOnce() -> E,
{
    type Item = I;
    type Error = E;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.stream.poll() {
            Ok(Async::Ready(Some(item))) => Ok(Async::Ready(item)),
            Ok(Async::Ready(None)) => {
                let or_else = self
                    .or_else
                    .take()
                    .expect("must not be polled after completion");

                Err(or_else())
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}
