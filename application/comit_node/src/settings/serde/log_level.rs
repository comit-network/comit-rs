use log::LevelFilter;
use serde::{de, export::fmt, Deserializer};
use std::str::FromStr;

pub fn deserialize<'de, D>(deserializer: D) -> Result<LevelFilter, D::Error>
where
	D: Deserializer<'de>,
{
	struct Visitor;

	impl<'de> de::Visitor<'de> for Visitor {
		type Value = LevelFilter;

		fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
			formatter.write_str(
				"a log level (\"OFF\", \"ERROR\", \"WARN\", \"INFO\", \"DEBUG\", \"TRACE\")",
			)
		}

		fn visit_str<E>(self, value: &str) -> Result<LevelFilter, E>
		where
			E: de::Error,
		{
			LevelFilter::from_str(value).map_err(E::custom)
		}
	}

	deserializer.deserialize_str(Visitor)
}
