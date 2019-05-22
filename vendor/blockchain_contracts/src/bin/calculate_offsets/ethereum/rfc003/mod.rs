mod compile_contract;
pub mod contract;
mod metadata;
pub mod offset;
mod placeholder_config;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Hex(hex::FromHexError),
    PlaceholderNotFound,
    MalformedConfig(serde_json::Error),
    CaptureSolcBytecode,
    MalformedRegex(regex::Error),
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

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::MalformedConfig(e)
    }
}

impl From<regex::Error> for Error {
    fn from(e: regex::Error) -> Self {
        Error::MalformedRegex(e)
    }
}
