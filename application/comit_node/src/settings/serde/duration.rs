use serde::{de, export::fmt, Deserializer};
use std::time::Duration;

pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
	D: Deserializer<'de>,
{
	struct Visitor;

	impl<'de> de::Visitor<'de> for Visitor {
		type Value = Duration;

		fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
			formatter.write_str("a duration in seconds")
		}

		fn visit_u64<E>(self, value: u64) -> Result<Duration, E>
		where
			E: de::Error,
		{
			Ok(Duration::from_secs(value))
		}
	}

	deserializer.deserialize_u64(Visitor)
}
