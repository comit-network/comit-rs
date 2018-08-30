use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::{fmt, time::Duration};

#[derive(Debug, Clone, Copy)]
pub struct Seconds {
    duration: Duration,
}

impl Seconds {
    pub const fn new(seconds: u64) -> Self {
        Seconds {
            duration: Duration::from_secs(seconds),
        }
    }
}

impl From<Duration> for Seconds {
    fn from(duration: Duration) -> Self {
        Seconds { duration }
    }
}

impl Into<Duration> for Seconds {
    fn into(self) -> Duration {
        self.duration
    }
}

impl<'de> Deserialize<'de> for Seconds {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = Seconds;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                formatter.write_str("A unsigned 64-bit integer representing a seconds duration")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Seconds, E>
            where
                E: de::Error,
            {
                Ok(Seconds {
                    duration: Duration::from_secs(v),
                })
            }
        }

        deserializer.deserialize_u64(Visitor)
    }
}

impl Serialize for Seconds {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.duration.as_secs())
    }
}
