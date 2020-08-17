use bitcoin::hashes::core::fmt::Formatter;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::{fmt, time::SystemTime};
use time::Duration;
use tracing::warn;

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

    /// Adds a duration to self using saturating add (or saturating sub if rhs
    /// is negative). Precision is seconds only i.e., nanoseconds are ignored.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn add_duration(self, rhs: Duration) -> Timestamp {
        if rhs.is_negative() {
            let abs = rhs.abs();
            return self.sub_duration(abs);
        }

        let seconds = rhs.whole_seconds();
        if seconds > u32::MAX as i64 {
            // This does not actually matter because we use saturating_add(), if we hit this
            // however we probably have a bug at the call site.
            warn!("duration is too big, truncation occurred while casting to u32");
        }
        let seconds = seconds as u32;

        self.plus(seconds)
    }

    /// Subtracts a duration from self using saturating sub (or saturating add
    /// if rhs is negative). Precision is seconds only i.e., nanoseconds are
    /// ignored.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn sub_duration(self, rhs: Duration) -> Timestamp {
        if rhs.is_negative() {
            let abs = rhs.abs();
            return self.add_duration(abs);
        }

        let seconds = rhs.whole_seconds();
        if seconds > u32::MAX as i64 {
            // This does not actually matter because we use saturating_sub(), if we hit this
            // however we probably have a bug at the call site.
            warn!("duration is too big, truncation occurred while casting to u32");
        }
        let seconds = seconds as u32;

        self.minus(seconds)
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

/// Return the duration between to timestamps.
pub fn duration_between(t: Timestamp, u: Timestamp) -> Duration {
    let t = t.0 as i64;
    let u = u.0 as i64;

    let seconds = u - t;
    Duration::new(seconds, 0)
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
