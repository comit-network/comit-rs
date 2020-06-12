use bitcoin::hashes::core::fmt::Formatter;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::{fmt, time::SystemTime};

/// An exact time and date used to represent absolute timelocks
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Timestamp(u32);

impl Timestamp {
    // This will work for the next 20 years
    #[allow(clippy::cast_possible_truncation)]
    pub fn now() -> Self {
        Timestamp(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("SystemTime::duration_since failed")
                .as_secs() as u32,
        )
    }

    pub fn plus(self, seconds: u32) -> Self {
        Self(self.0.saturating_add(seconds))
    }

    pub fn minus(self, seconds: u32) -> Self {
        Self(self.0.saturating_sub(seconds))
    }

    pub fn to_bytes(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

/// The u32 input is the number of seconds since epoch
impl From<u32> for Timestamp {
    fn from(item: u32) -> Self {
        Self(item)
    }
}

/// The u32 returned is the number of seconds since epoch
impl From<Timestamp> for u32 {
    fn from(item: Timestamp) -> Self {
        item.0
    }
}

/// The i64 returned is the number of seconds since epoch
impl From<Timestamp> for i64 {
    fn from(item: Timestamp) -> Self {
        i64::from(item.0)
    }
}

impl From<Timestamp> for NaiveDateTime {
    fn from(t: Timestamp) -> Self {
        let secs = i64::from(t);
        NaiveDateTime::from_timestamp(secs, 0)
    }
}

impl From<crate::ethereum::U256> for Timestamp {
    fn from(value: crate::ethereum::U256) -> Self {
        value.low_u32().into()
    }
}

/// A duration used to represent a relative timelock
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct RelativeTime(u32);

impl RelativeTime {
    pub const fn new(time_secs: u32) -> Self {
        RelativeTime(time_secs)
    }

    pub fn to_bytes(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

/// The u32 returned is the duration in seconds
impl From<RelativeTime> for u32 {
    fn from(item: RelativeTime) -> Self {
        item.0
    }
}

/// The u32 input is the duration in seconds
impl From<u32> for RelativeTime {
    fn from(item: u32) -> Self {
        Self(item)
    }
}

impl fmt::Display for RelativeTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
