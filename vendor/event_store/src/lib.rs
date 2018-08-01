use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::RwLock,
};

pub trait Event: Clone + 'static {
    type Prev: Event;
}

impl Event for () {
    type Prev = ();
}

#[derive(Debug)]
pub enum Error<E: Event> {
    PrevEventMissing,
    DuplicateEvent(E),
}

pub trait EventStore<K> {
    fn add_event<E: Event>(&self, key: K, event: E) -> Result<(), Error<E>>;
    fn get_event<E: Event>(&self, key: K) -> Option<E>;
}

pub struct InMemoryEventStore<K: Hash + Eq> {
    events: RwLock<HashMap<(TypeId, K), Box<Any>>>,
}

impl<K: Hash + Eq> InMemoryEventStore<K> {
    pub fn new() -> Self {
        InMemoryEventStore {
            events: RwLock::new(HashMap::new()),
        }
    }

    fn _get_event<E: Event>(
        events: &HashMap<(TypeId, K), Box<Any>>,
        type_id: TypeId,
        key: K,
    ) -> Option<E> {
        events.get(&(type_id, key)).map(|event| {
            let _any: &Any = event.borrow();
            _any.downcast_ref::<E>().unwrap().clone()
        })
    }
}

impl<K: Hash + Eq + Clone> EventStore<K> for InMemoryEventStore<K> {
    fn add_event<E: Event>(&self, key: K, event: E) -> Result<(), Error<E>> {
        let unit_type_id: TypeId = TypeId::of::<()>();

        let id = TypeId::of::<E>();
        let id_prev = TypeId::of::<E::Prev>();

        let mut events = self.events.write().unwrap();
        let get_prev_event = Self::_get_event::<E::Prev>(&*events, id_prev, key.clone());

        if get_prev_event.is_none() && id_prev != unit_type_id {
            return Err(Error::PrevEventMissing);
        }

        let get_existing_event = Self::_get_event::<E>(&*events, id, key.clone());

        if let Some(existing) = get_existing_event {
            return Err(Error::DuplicateEvent(existing));
        }

        events.insert((id, key), Box::new(event));
        Ok(())
    }

    fn get_event<E: Event>(&self, key: K) -> Option<E> {
        let id = TypeId::of::<E>();
        let events = self.events.read().unwrap();
        Self::_get_event::<E>(&*events, id, key)
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
        let event_store = InMemoryEventStore::new();
        assert!(event_store.add_event(&42, Init {}).is_ok());
        assert_eq!(event_store.get_event::<Init>(&42).unwrap(), Init {});
        assert!(event_store.get_event::<Init>(&32).is_none());
    }

    #[test]
    fn add_event_without_dependent_event() {
        #[derive(PartialEq, Debug, Clone)]
        struct Second {}

        impl Event for Second {
            type Prev = Init;
        }
        let event_store = InMemoryEventStore::new();

        assert!(event_store.add_event(&42, Second {}).is_err());

        event_store.add_event(&42, Init {}).unwrap();
        assert!(event_store.add_event(&42, Second {}).is_ok())
    }

    #[test]
    fn add_event_twice_fails() {
        let event_store = InMemoryEventStore::new();
        event_store.add_event(&42, Init {}).unwrap();
        assert!(event_store.add_event(&42, Init {}).is_err());
    }
}
