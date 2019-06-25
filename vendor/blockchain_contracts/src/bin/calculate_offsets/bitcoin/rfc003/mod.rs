use crate::calculate_offsets::{
    concat_path, metadata::Metadata, placeholder_config::PlaceholderConfig, Contract,
};
use std::ffi::OsStr;

pub struct BitcoinScript {
    bytes: Vec<u8>,
    placeholder_config: PlaceholderConfig,
}

#[derive(Debug)]
pub enum Error {
    PlaceholderNotFound,
    Hex(hex::FromHexError),
    MalformedConfig(serde_json::Error),
    IO(std::io::Error),
}

impl From<crate::calculate_offsets::Error> for Error {
    fn from(err: crate::calculate_offsets::Error) -> Self {
        match err {
            crate::calculate_offsets::Error::PlaceholderNotFound => Error::PlaceholderNotFound,
            crate::calculate_offsets::Error::Hex(err) => Error::Hex(err),
        }
    }
}

impl From<crate::calculate_offsets::placeholder_config::Error> for Error {
    fn from(err: crate::calculate_offsets::placeholder_config::Error) -> Self {
        use crate::calculate_offsets::placeholder_config::Error as PlaceholderError;
        match err {
            PlaceholderError::IO(err) => Error::IO(err),
            PlaceholderError::MalformedConfig(err) => Error::MalformedConfig(err),
        }
    }
}

impl Contract for BitcoinScript {
    type Error = Error;

    fn compile<S: AsRef<OsStr>>(template_folder: S) -> Result<Self, Self::Error> {
        let bytes = hex::decode("6382012088a82010000000000000000000000000000000000000000000000000000000000000018876a9143000000000000000000000000000000000000003670420000002b17576a91440000000000000000000000000000000000000046888ac").unwrap();
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

    fn bytes(&self) -> &Vec<u8> {
        &self.bytes
    }
}
