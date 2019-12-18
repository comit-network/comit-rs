mod initialize;

pub use self::initialize::initialize;

use std::fmt::Debug;

pub trait Scribe {
    fn scribe(&self) -> String
    where
        Self: Debug,
    {
        // Logging with Debug is fine unless its not.
        format!("{:?}", self)
    }
}

impl Scribe for bitcoin::blockdata::block::Block {}
impl Scribe for bitcoin::blockdata::transaction::Transaction {}
