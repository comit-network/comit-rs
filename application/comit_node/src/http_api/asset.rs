use ::serde::{de::DeserializeOwned, Deserializer, Serialize};
use std::{collections::BTreeMap, fmt};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HttpAsset {
    name: String,
    #[serde(default, flatten)]
    parameters: BTreeMap<String, serde_json::Value>,
}

impl HttpAsset {
    pub fn is_asset(&self, name: &'static str) -> Result<(), Error> {
        if self.name == name {
            Ok(())
        } else {
            Err(Error::WrongAsset)
        }
    }

    pub fn parameter<P: DeserializeOwned>(&mut self, key: &'static str) -> Result<P, Error> {
        let parameter = self
            .parameters
            .remove(key)
            .ok_or(Error::ParameterNotFound)?;

        serde_json::from_value(parameter).map_err(Error::Serde)
    }

    pub fn with_asset(name: &'static str) -> HttpAsset {
        HttpAsset {
            name: String::from(name),
            parameters: BTreeMap::new(),
        }
    }

    pub fn with_parameter<P: Serialize>(
        self,
        key: &'static str,
        parameter: P,
    ) -> Result<HttpAsset, Error> {
        let HttpAsset {
            name,
            mut parameters,
        } = self;

        parameters.insert(
            String::from(key),
            serde_json::to_value(&parameter).map_err(Error::Serde)?,
        );

        Ok(HttpAsset { name, parameters })
    }
}

#[derive(Debug)]
pub enum Error {
    WrongAsset,
    ParameterNotFound,
    Serde(serde_json::Error),
    Parsing,
}

impl fmt::Display for Error {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}

macro_rules! _impl_from_http_quantity_asset {
    ($asset_type:ty, $name:ident) => {
        impl FromHttpAsset for $asset_type {
            #[allow(unused_mut)]
            fn from_http_asset(mut asset: HttpAsset) -> Result<Self, asset::Error> {
                asset.is_asset(stringify!($name))?;

                asset.parameter("quantity")
            }
        }
    };
}

macro_rules! impl_from_http_quantity_asset {
    ($asset_type:ty, $name:ident) => {
        _impl_from_http_quantity_asset!($asset_type, $name);
    };
}

pub trait FromHttpAsset
where
    Self: Sized,
{
    fn from_http_asset(asset: HttpAsset) -> Result<Self, Error>;
}

impl FromHttpAsset for HttpAsset {
    fn from_http_asset(asset: HttpAsset) -> Result<Self, Error> {
        Ok(asset)
    }
}

pub mod serde {

    use super::*;

    pub fn deserialize<'de, D, T: FromHttpAsset>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        use ::serde::{de::Error, Deserialize};
        let asset = HttpAsset::deserialize(deserializer)?;

        T::from_http_asset(asset).map_err(D::Error::custom)
    }
}
