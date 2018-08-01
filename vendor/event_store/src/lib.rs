use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    ops::Deref,
    sync::RwLock,
};

pub trait Event: Clone + 'static {}

#[derive(Debug)]
pub enum Error {
    InvalidState,
}

pub trait EventStore<K> {
    fn add_event<E: Event>(&self, key: K, event: E) -> Result<(), Error>;
    fn get_event<E: Event>(&self, key: K) -> Result<E, Error>;
}

pub struct InMemoryEventStore<T: Hash + Eq> {
    events: RwLock<HashMap<(TypeId, T), Box<Any>>>,
}

impl<T: Hash + Eq> InMemoryEventStore<T> {
    pub fn new() -> Self {
        InMemoryEventStore {
            events: RwLock::new(HashMap::new()),
        }
    }
}

impl<K: Hash + Eq> EventStore<K> for InMemoryEventStore<K> {
    fn add_event<E: Event>(&self, key: K, event: E) -> Result<(), Error> {
        let id = TypeId::of::<E>();
        let mut events = self.events.write().unwrap();
        events.insert((id, key), Box::new(event));
        Ok(())
    }

    fn get_event<E: Event>(&self, key: K) -> Result<E, Error> {
        let id = TypeId::of::<E>();
        let events = self.events.read().unwrap();

        events
            .get(&(id, key))
            .map(|event| {
                let _any: &Any = event.borrow();
                _any.downcast_ref::<E>().unwrap().clone()
            })
            .ok_or(Error::InvalidState)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(PartialEq, Debug, Clone)]
    struct Init {}

    impl Event for Init {}
    #[test]
    fn add_single_event() {
        let event_store = InMemoryEventStore::new();
        assert!(event_store.add_event(&42, Init {}).is_ok());
        assert_eq!(event_store.get_event::<Init>(&42).unwrap(), Init {});
    }
}
