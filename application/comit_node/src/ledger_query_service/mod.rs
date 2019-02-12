pub use self::{bitcoin::*, client::*, ethereum::*};
use crate::swap_protocols::ledger::Ledger;
use reqwest::Url;
use serde::Serialize;
use std::{fmt::Debug, hash::Hash, marker::PhantomData};

mod bitcoin;
mod client;
mod ethereum;

mod timer_poll_future;

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub struct QueryId<L: Ledger> {
    location: Url,
    ledger_type: PhantomData<L>,
}

impl<L: Ledger> AsRef<Url> for QueryId<L> {
    fn as_ref(&self) -> &Url {
        &self.location
    }
}

impl<L: Ledger> QueryId<L> {
    pub fn new(location: Url) -> Self {
        QueryId {
            location,
            ledger_type: PhantomData,
        }
    }
}

#[derive(Fail, Debug, PartialEq, Clone)]
pub enum Error {
    #[fail(display = "The request failed to send.")]
    FailedRequest(String),
    #[fail(display = "The response was somehow malformed.")]
    MalformedResponse(String),
}

pub trait Query: Sized + Clone + Debug + Send + Sync + Eq + Hash + Serialize + 'static {}
