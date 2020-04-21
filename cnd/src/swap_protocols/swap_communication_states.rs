use crate::swap_protocols::{
    rfc003::SwapId,
    state::{Get, Insert},
};
use async_trait::async_trait;
use std::{any::Any, clone::Clone, collections::HashMap};
use tokio::sync::Mutex;

#[derive(Default, Debug)]
pub struct SwapCommunicationStates {
    states: Mutex<HashMap<SwapId, Box<dyn Any + Send>>>,
}

#[async_trait]
impl<S> Insert<S> for SwapCommunicationStates
where
    S: Send + 'static,
{
    async fn insert(&self, key: SwapId, value: S) {
        let mut states = self.states.lock().await;
        states.insert(key, Box::new(value));
    }
}

#[async_trait]
impl<S> Get<S> for SwapCommunicationStates
where
    S: Clone + Send + 'static,
{
    async fn get(&self, key: &SwapId) -> anyhow::Result<Option<S>> {
        let states = self.states.lock().await;
        match states.get(key) {
            Some(state) => match state.downcast_ref::<S>() {
                Some(state) => Ok(Some(state.clone())),
                None => Err(anyhow::anyhow!("invalid type")),
            },
            None => Ok(None),
        }
    }
}
