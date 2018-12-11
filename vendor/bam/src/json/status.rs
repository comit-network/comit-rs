use crate::api::Status;
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

impl Serialize for Status {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        match self {
            Status::OK(code) => serializer.serialize_str(format!("OK{:02}", code).as_str()),
            Status::SE(code) => serializer.serialize_str(format!("SE{:02}", code).as_str()),
            Status::RE(code) => serializer.serialize_str(format!("RE{:02}", code).as_str()),
        }
    }
}

struct StatusVisitor;

impl<'de> Visitor<'de> for StatusVisitor {
    type Value = Status;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(formatter, "A status code like 'OK00', 'SE00' or 'RE00'")
    }

    fn visit_str<E>(self, v: &str) -> Result<Status, E>
    where
        E: Error,
    {
        let (status_family, code) = v.split_at(2);

        let code = code.parse().map_err(Error::custom)?;

        let status = match status_family {
            "OK" => Status::OK(code),
            "SE" => Status::SE(code),
            "RE" => Status::RE(code),
            _ => {
                return Err(Error::custom(format!(
                    "Unknown status family: {}",
                    status_family
                )));
            }
        };

        Ok(status)
    }
}

impl<'de> Deserialize<'de> for Status {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StatusVisitor {})
    }
}
