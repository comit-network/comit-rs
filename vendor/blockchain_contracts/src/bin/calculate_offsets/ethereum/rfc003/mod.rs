mod compile_contract;
pub mod contract;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Hex(hex::FromHexError),
    PlaceholderNotFound,
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
