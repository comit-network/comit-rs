use serde::{de, export::fmt, Deserializer};
use url::Url;

pub fn deserialize<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Url;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an url")
        }

        fn visit_str<E>(self, value: &str) -> Result<Url, E>
        where
            E: de::Error,
        {
            Url::parse(value).map_err(E::custom)
        }
    }

    deserializer.deserialize_str(Visitor)
}
