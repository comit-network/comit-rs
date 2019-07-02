use crate::calculate_offsets::{self, placeholder_config};

mod compile_contract;
pub mod contract;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Hex(hex::FromHexError),
    PlaceholderNotFound(String),
    MalformedConfig(serde_json::Error),
    CaptureSolcBytecode,
    MalformedRegex(regex::Error),
    NumberConversionFailed(std::num::TryFromIntError),
}

impl From<hex::FromHexError> for Error {
    fn from(e: hex::FromHexError) -> Self {
        Error::Hex(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO(e)
    }
}

impl From<regex::Error> for Error {
    fn from(e: regex::Error) -> Self {
        Error::MalformedRegex(e)
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(e: std::num::TryFromIntError) -> Self {
        Error::NumberConversionFailed(e)
    }
}

impl From<calculate_offsets::Error> for Error {
    fn from(err: calculate_offsets::Error) -> Self {
        match err {
            calculate_offsets::Error::PlaceholderNotFound(placeholder) => {
                Error::PlaceholderNotFound(placeholder)
            }
            calculate_offsets::Error::Hex(err) => Error::Hex(err),
        }
    }
}

impl From<placeholder_config::Error> for Error {
    fn from(err: placeholder_config::Error) -> Self {
        match err {
            placeholder_config::Error::IO(err) => Error::IO(err),
            placeholder_config::Error::MalformedConfig(err) => Error::MalformedConfig(err),
        }
    }
}
