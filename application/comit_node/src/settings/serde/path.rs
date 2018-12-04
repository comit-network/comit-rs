use serde::{de, export::fmt, Deserializer};
use shellexpand;
use std::path::{Path, PathBuf};

pub fn deserialize<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = PathBuf;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a local file path")
        }

        fn visit_str<E>(self, value: &str) -> Result<PathBuf, E>
        where
            E: de::Error,
        {
            let expanded = shellexpand::full(value).map_err(E::custom)?;
            let path = Path::new(&*expanded);
            // Using PathBuf as Path is not sized
            Ok(PathBuf::from(path))
        }
    }

    deserializer.deserialize_str(Visitor)
}
