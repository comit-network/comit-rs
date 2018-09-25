use serde::{de, export::fmt, Deserializer};
use std::net::SocketAddr;

pub fn deserialize<'de, D>(deserializer: D) -> Result<SocketAddr, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = SocketAddr;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an ip address with port")
        }

        fn visit_str<E>(self, value: &str) -> Result<SocketAddr, E>
        where
            E: de::Error,
        {
            value.parse().map_err(E::custom)
        }
    }

    deserializer.deserialize_str(Visitor)
}
