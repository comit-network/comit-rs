use ::serde::{de::DeserializeOwned, Deserializer, Serialize};
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HttpLedger {
    name: String,
    #[serde(default, flatten)]
    parameters: HashMap<String, serde_json::Value>,
}

impl HttpLedger {
    pub fn is_ledger(&self, name: &str) -> Result<(), Error> {
        if self.name == name {
            Ok(())
        } else {
            Err(Error::WrongLedger)
        }
    }

    pub fn parameter<P: DeserializeOwned>(&mut self, key: &'static str) -> Result<P, Error> {
        let parameter = self
            .parameters
            .remove(key)
            .ok_or(Error::ParameterNotFound)?;

        serde_json::from_value(parameter).map_err(Error::Serde)
    }

    pub fn with_ledger(name: &'static str) -> HttpLedger {
        HttpLedger {
            name: String::from(name),
            parameters: HashMap::new(),
        }
    }

    pub fn with_parameter<P: Serialize>(
        self,
        key: &'static str,
        parameter: P,
    ) -> Result<HttpLedger, Error> {
        let HttpLedger {
            name,
            mut parameters,
        } = self;

        parameters.insert(
            String::from(key),
            serde_json::to_value(&parameter).map_err(Error::Serde)?,
        );

        Ok(HttpLedger { name, parameters })
    }
}

#[derive(Debug)]
pub enum Error {
    WrongLedger,
    ParameterNotFound,
    Serde(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}

macro_rules! _impl_from_http_ledger {
    (
        $name:ident {
            $($fields:ident),*
        }
    ) => {
        impl FromHttpLedger for $name {
            #[allow(unused_mut)]
            fn from_http_ledger(mut ledger: HttpLedger) -> Result<Self, ledger::Error> {
                let name = stringify!($name).to_lowercase();
                ledger.is_ledger(name.as_ref())?;

                Ok($name {
                    $(
                        $fields: ledger.parameter(stringify!($fields))?,
                    )*
                })
            }
        }
    };
}

macro_rules! impl_from_http_ledger {
    (
        $name:ident
    ) => {
        _impl_from_http_ledger!($name {});
    };
    (
        $name:ident {
            $($fields:ident),*
        }
    ) => {
        _impl_from_http_ledger!($name {$($fields),*});
    };
}

pub trait FromHttpLedger
where
    Self: Sized,
{
    fn from_http_ledger(ledger: HttpLedger) -> Result<Self, Error>;
}

impl FromHttpLedger for HttpLedger {
    fn from_http_ledger(ledger: HttpLedger) -> Result<Self, Error> {
        Ok(ledger)
    }
}

pub mod serde {

    use super::*;

    pub fn deserialize<'de, D, T: FromHttpLedger>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        use ::serde::{de::Error, Deserialize};
        let ledger = HttpLedger::deserialize(deserializer)?;

        T::from_http_ledger(ledger).map_err(D::Error::custom)
    }

}
