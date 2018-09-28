#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    collections::{hash_map::RandomState, HashMap, HashSet},
    hash::Hash,
    iter::FromIterator,
    sync::Mutex,
};

pub trait Event: Clone + 'static + Send + Sync {
    type Prev: Event;
}

impl Event for () {
    type Prev = ();
}

#[derive(Debug)]
pub enum Error {
    PrevEventMissing,
    DuplicateEvent,
    NotFound,
}

pub trait EventStore<K> {
    fn add_event<E: Event>(&self, key: K, event: E) -> Result<(), Error>;
    fn get_event<E: Event>(&self, key: K) -> Result<E, Error>;
}

#[derive(Default, Debug)]
pub struct InMemoryEventStore<K: Hash + Eq> {
    events: Mutex<HashMap<(TypeId, K), Box<Any + Send>>>,
}

impl<K: Hash + Eq + Clone> InMemoryEventStore<K> {
    fn _get_event<E: Event>(events: &HashMap<(TypeId, K), Box<Any + Send>>, key: K) -> Option<E> {
        let key = (TypeId::of::<E>(), key);

        events.get(&key).map(|event| {
            let _any: &(Any + Send) = event.borrow();
            _any.downcast_ref::<E>().unwrap().clone()
        })
    }

    fn _add_event<E: Event>(events: &mut HashMap<(TypeId, K), Box<Any + Send>>, key: K, event: E) {
        let key = (TypeId::of::<E>(), key);
        let value = Box::new(event);

        let old_event = events.insert(key, value);
        debug_assert!(old_event.is_none());
    }

    pub fn keys(&self) -> impl Iterator<Item = K> {
        let events = self.events.lock().unwrap();
        // get all the unique ids
        HashSet::<K, RandomState>::from_iter(events.keys().map(|e| e.1.clone())).into_iter()
    }

    fn is_initial_event<E: Event>() -> bool {
        TypeId::of::<E>() == TypeId::of::<()>()
    }
}

impl<K: Hash + Eq + Clone> EventStore<K> for InMemoryEventStore<K> {
    fn add_event<E: Event>(&self, key: K, event: E) -> Result<(), Error> {
        let mut events = self.events.lock().unwrap();

        let prev_event_is_missing = Self::_get_event::<E::Prev>(&*events, key.clone()).is_none();
        let prev_event_is_initial = Self::is_initial_event::<E::Prev>();

        if prev_event_is_missing && !prev_event_is_initial {
            return Err(Error::PrevEventMissing);
        }

        let existing_event = Self::_get_event::<E>(&*events, key.clone());

        if existing_event.is_some() {
            return Err(Error::DuplicateEvent);
        }

        Self::_add_event(&mut events, key, event);

        Ok(())
    }

    fn get_event<E: Event>(&self, key: K) -> Result<E, Error> {
        let events = self.events.lock().unwrap();
        Self::_get_event::<E>(&*events, key).ok_or(Error::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(PartialEq, Debug, Clone)]
    struct Init {}

    impl Event for Init {
        type Prev = ();
    }
    #[test]
    fn add_single_event() {
        let event_store = InMemoryEventStore::default();
        assert!(event_store.add_event(42, Init {}).is_ok());
        assert_eq!(event_store.get_event::<Init>(42).unwrap(), Init {});
        assert!(event_store.get_event::<Init>(32).is_err());
    }

    #[test]
    fn add_event_without_dependent_event() {
        #[derive(PartialEq, Debug, Clone)]
        struct Second {}

        impl Event for Second {
            type Prev = Init;
        }
        let event_store = InMemoryEventStore::default();

        assert!(event_store.add_event(42, Second {}).is_err());

        event_store.add_event(42, Init {}).unwrap();
        assert!(event_store.add_event(42, Second {}).is_ok())
    }

    #[test]
    fn add_event_twice_fails() {
        let event_store = InMemoryEventStore::default();
        event_store.add_event(42, Init {}).unwrap();
        assert!(event_store.add_event(42, Init {}).is_err());
    }
}
