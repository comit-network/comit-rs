use crate::calculate_offsets::{
    self, concat_path,
    metadata::Metadata,
    placeholder_config::{self, PlaceholderConfig},
    Contract,
};
use std::{ffi::OsStr, path::Path, string::FromUtf8Error};

mod compile_contract;

pub struct BitcoinScript {
    bytes: Vec<u8>,
    placeholder_config: PlaceholderConfig,
}

#[derive(Debug)]
pub enum Error {
    CalculateOffset(calculate_offsets::Error),
    PlaceholderConfig(placeholder_config::Error),
    Hex(hex::FromHexError),
    IO(std::io::Error),
    MalformedRegex(regex::Error),
    MalformedInput(FromUtf8Error),
    Miniscript(miniscript::Error),
}

impl From<calculate_offsets::Error> for Error {
    fn from(err: calculate_offsets::Error) -> Self {
        Error::CalculateOffset(err)
    }
}

impl From<placeholder_config::Error> for Error {
    fn from(err: placeholder_config::Error) -> Self {
        Error::PlaceholderConfig(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err)
    }
}

impl From<regex::Error> for Error {
    fn from(e: regex::Error) -> Self {
        Error::MalformedRegex(e)
    }
}

impl From<hex::FromHexError> for Error {
    fn from(e: hex::FromHexError) -> Self {
        Error::Hex(e)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Self {
        Error::MalformedInput(err)
    }
}

impl From<miniscript::Error> for Error {
    fn from(err: miniscript::Error) -> Self {
        Error::Miniscript(err)
    }
}

impl Contract for BitcoinScript {
    type Error = Error;

    fn compile<S: AsRef<OsStr>>(template_folder: S) -> Result<Self, Self::Error> {
        let bytes = compile_contract::compile(Path::new(&template_folder).join("contract.script"))?;
        let placeholder_config =
            PlaceholderConfig::from_file(concat_path(&template_folder, "config.json"))?;

        Ok(Self {
            bytes,
            placeholder_config,
        })
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            ledger_name: self.placeholder_config.ledger_name.to_owned(),
            asset_name: self.placeholder_config.asset_name.to_owned(),
            contract: self.bytes.to_owned(),
        }
    }

    fn placeholder_config(&self) -> &PlaceholderConfig {
        &self.placeholder_config
    }

    fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}
