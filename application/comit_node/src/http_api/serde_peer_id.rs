use libp2p::PeerId;
use serde::{de, export::fmt, Deserializer};

pub fn deserialize<'de, D>(deserializer: D) -> Result<PeerId, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = PeerId;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a peer id")
        }

        fn visit_str<E>(self, value: &str) -> Result<PeerId, E>
        where
            E: de::Error,
        {
            value.parse().map_err(E::custom)
        }
    }

    deserializer.deserialize_str(Visitor)
}
