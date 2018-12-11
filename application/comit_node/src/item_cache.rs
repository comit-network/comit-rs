use futures::sync::oneshot;
use std::fmt::{self, Debug, Formatter};
use tokio::prelude::*;

enum ItemOrFuture<T, E> {
    Item(T),
    Future(Box<dyn Future<Item = T, Error = E> + Send + 'static>),
}

impl<T: Debug, E> Debug for ItemOrFuture<T, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            ItemOrFuture::Item(i) => write!(f, "ItemOrFuture::Item({:?})", i),
            ItemOrFuture::Future(_) => write!(f, "ItemOrFuture::Future"),
        }
    }
}

#[derive(Debug)]
pub struct ItemCache<T, E> {
    inner: ItemOrFuture<T, E>,
}

impl<T, E> From<ItemOrFuture<T, E>> for ItemCache<T, E> {
    fn from(item_or_future: ItemOrFuture<T, E>) -> Self {
        ItemCache {
            inner: item_or_future,
        }
    }
}

impl<T, E> ItemCache<T, E> {
    pub fn from_item(item: T) -> Self {
        Self {
            inner: ItemOrFuture::Item(item),
        }
    }

    pub fn from_future<F>(future: F) -> Self
    where
        F: Future<Item = T, Error = E> + Send + 'static,
    {
        Self {
            inner: ItemOrFuture::Future(Box::new(future)),
        }
    }
}

impl<T: Clone, E> Future for ItemCache<T, E> {
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Result<Async<<Self as Future>::Item>, <Self as Future>::Error> {
        let item = match self.inner {
            ItemOrFuture::Item(ref item) => item.clone(),
            ItemOrFuture::Future(ref mut future) => try_ready!(future.poll()),
        };

        if let ItemOrFuture::Future(_) = self.inner {
            self.inner = ItemOrFuture::Item(item.clone())
        }

        Ok(Async::Ready(item))
    }
}

impl<T: Clone + Debug + Send + 'static, E: Clone + Debug + Send + 'static> ItemCache<T, E> {
    pub fn duplicate(self) -> (Self, Self) {
        let (first, second) = match self.inner {
            ItemOrFuture::Item(item) => {
                (ItemOrFuture::Item(item.clone()), ItemOrFuture::Item(item))
            }
            ItemOrFuture::Future(future) => {
                let (item_sender, item_receiver) = oneshot::channel();
                let (error_sender, error_receiver) = oneshot::channel();

                let composed = future
                    .and_then(|item| {
                        let _ = item_sender.send(item.clone());
                        Ok(item)
                    })
                    .map_err(|e| {
                        let _ = error_sender.send(e.clone());
                        e
                    });

                let copy = item_receiver
                    .select2(error_receiver)
                    .map_err(|_| panic!("senders went away"))
                    .and_then(|either| match either {
                        future::Either::A((item, _)) => Ok(item),
                        future::Either::B((error, _)) => Err(error),
                    });

                (
                    ItemOrFuture::Future(Box::new(composed)),
                    ItemOrFuture::Future(Box::new(copy)),
                )
            }
        };

        (first.into(), second.into())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use futures;

    #[test]
    fn polling_sender_of_oneshot_twice_results_in_error() {
        let (sender, mut receiver) = oneshot::channel();

        sender.send(42).unwrap();

        assert_eq!(receiver.poll(), Ok(Async::Ready(42)));
        assert_eq!(receiver.poll(), Err(futures::Canceled));
    }

    #[test]
    fn item_or_future_caches_item_after_resolving() {
        let (sender, receiver) = oneshot::channel();

        let mut item_cache: ItemCache<i32, futures::Canceled> =
            ItemOrFuture::Future(Box::new(receiver)).into();

        sender.send(42).unwrap();

        assert_eq!(item_cache.poll(), Ok(Async::Ready(42)));
        assert_eq!(item_cache.poll(), Ok(Async::Ready(42)));
    }

    #[test]
    fn given_duplicated_item_cache_when_resolved_both_can_poll() {
        let (sender, receiver) = oneshot::channel();

        let item_cache: ItemCache<i32, futures::Canceled> =
            ItemOrFuture::Future(Box::new(receiver)).into();

        let (mut first, mut second) = item_cache.duplicate();

        sender.send(42).unwrap();

        assert_eq!(first.poll(), Ok(Async::Ready(42)));
        assert_eq!(second.poll(), Ok(Async::Ready(42)));
    }

}
