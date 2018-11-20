use serde::{de::DeserializeOwned, Deserializer, Serialize, Serializer};
use serde_json;
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpAsset {
    name: String,
    #[serde(default, flatten)]
    parameters: HashMap<String, serde_json::Value>,
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
            parameters: HashMap::new(),
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

macro_rules! impl_from_http_quantity_asset {
    ($asset_type:ty, $name:ident) => {
        impl FromHttpAsset for $asset_type {
            #[allow(unused_mut)]
            fn from_http_asset(mut asset: HttpAsset) -> Result<Self, Error> {
                let _ = asset.is_asset(stringify!($name))?;

                asset.parameter("quantity")
            }
        }
    };
}

macro_rules! impl_to_http_quantity_asset {
    ($asset_type:ty, $name:ident) => {
        impl ToHttpAsset for $asset_type {
            fn to_http_asset(&self) -> Result<HttpAsset, Error> {
                Ok(HttpAsset::with_asset(stringify!($name)).with_parameter("quantity", &self)?)
            }
        }
    };
}

macro_rules! impl_http_quantity_asset {
    ($asset_type:ty, $name:ident) => {
        impl_from_http_quantity_asset!($asset_type, $name);
        impl_to_http_quantity_asset!($asset_type, $name);
    };
}

pub trait FromHttpAsset
where
    Self: Sized,
{
    fn from_http_asset(asset: HttpAsset) -> Result<Self, Error>;
}

pub trait ToHttpAsset
where
    Self: Sized,
{
    fn to_http_asset(&self) -> Result<HttpAsset, Error>;
}

impl FromHttpAsset for HttpAsset {
    fn from_http_asset(asset: HttpAsset) -> Result<Self, Error> {
        Ok(asset)
    }
}

impl ToHttpAsset for HttpAsset {
    fn to_http_asset(&self) -> Result<HttpAsset, Error> {
        Ok(self.clone())
    }
}

pub mod serde {

    use super::*;

    pub fn deserialize<'de, D, T: FromHttpAsset>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::{de::Error, Deserialize};

        let asset = HttpAsset::deserialize(deserializer)?;

        T::from_http_asset(asset).map_err(D::Error::custom)
    }

    pub fn serialize<T: ToHttpAsset, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::{ser::Error, Serialize};

        let asset = value.to_http_asset().map_err(S::Error::custom)?;

        HttpAsset::serialize(&asset, serializer)
    }
}
