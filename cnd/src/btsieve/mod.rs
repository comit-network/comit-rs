pub use self::{bitcoin::*, client::*, ethereum::*};
use crate::swap_protocols::ledger::Ledger;
use failure::Fail;
use reqwest::Url;
use serde::Serialize;
use std::{fmt::Debug, hash::Hash, marker::PhantomData};

mod bitcoin;
mod client;
mod ethereum;
mod poll_until_item;

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
    #[fail(display = "Btsieve returned a failure.")]
    ResponseFailure(String),
    #[fail(display = "The btsieve client encountered an unrecoverable internal error.")]
    Internal,
}

pub trait Query: Sized + Clone + Debug + Send + Sync + Eq + Hash + Serialize + 'static {
    fn query_id(&self) -> String;
}
