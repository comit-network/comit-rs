use serde::{de::DeserializeOwned, Deserializer, Serialize, Serializer};
use serde_json;
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    value: String,
    #[serde(default)]
    parameters: HashMap<String, serde_json::Value>,
}

impl Header {
    pub fn has_value(&self, value: &'static str) -> Result<(), Error> {
        if self.value == value {
            Ok(())
        } else {
            Err(Error::WrongValue)
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn parameter<P: DeserializeOwned>(&mut self, key: &'static str) -> Result<P, Error> {
        let parameter = self
            .parameters
            .remove(key)
            .ok_or(Error::ParameterNotFound)?;

        serde_json::from_value(parameter).map_err(Error::Serde)
    }

    pub fn with_value(value: &'static str) -> Header {
        Header {
            value: String::from(value),
            parameters: HashMap::new(),
        }
    }

    pub fn with_parameter<P: Serialize>(
        self,
        key: &'static str,
        parameter: P,
    ) -> Result<Header, Error> {
        let Header {
            value,
            mut parameters,
        } = self;

        parameters.insert(
            String::from(key),
            serde_json::to_value(&parameter).map_err(Error::Serde)?,
        );

        Ok(Header { value, parameters })
    }
}

#[derive(Debug)]
pub enum Error {
    WrongValue,
    ParameterNotFound,
    Serde(serde_json::Error),
    Parsing,
}

impl fmt::Display for Error {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}

pub trait FromBamHeader
where
    Self: Sized,
{
    fn from_bam_header(header: Header) -> Result<Self, Error>;
}

pub trait ToBamHeader
where
    Self: Sized,
{
    fn to_bam_header(&self) -> Result<Header, Error>;
}

impl FromBamHeader for Header {
    fn from_bam_header(header: Header) -> Result<Self, Error> {
        Ok(header)
    }
}

impl ToBamHeader for Header {
    fn to_bam_header(&self) -> Result<Header, Error> {
        Ok(self.clone())
    }
}

pub mod serde {

    use super::*;

    pub fn deserialize<'de, D, T: FromBamHeader>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::{de::Error, Deserialize};

        let header = Header::deserialize(deserializer)?;

        T::from_bam_header(header).map_err(D::Error::custom)
    }

    pub fn serialize<T: ToBamHeader, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::{ser::Error, Serialize};

        let header = value.to_bam_header().map_err(S::Error::custom)?;

        Header::serialize(&header, serializer)
    }
}
