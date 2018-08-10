use hex;
use std::{
    fs::File,
    io::{self, prelude::*},
    path::Path,
};
use FromFile;

#[derive(Debug)]
pub struct Macaroon(Vec<u8>);

#[derive(Debug)]
pub enum ReadMacaroonError {
    OpenFileFail(io::Error),
    ReadFileFail(io::Error),
}

impl Macaroon {
    pub fn to_hex(&self) -> String {
        hex::encode(&self)
    }
}

impl From<Vec<u8>> for Macaroon {
    fn from(bytes: Vec<u8>) -> Self {
        Macaroon(bytes)
    }
}

impl AsRef<[u8]> for Macaroon {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl FromFile for Macaroon {
    type Err = ReadMacaroonError;

    fn from_file<P: AsRef<Path>>(file: P) -> Result<Self, Self::Err> {
        let mut f = File::open(file).or_else(|e| Err(ReadMacaroonError::OpenFileFail(e)))?;

        let mut buffer = Vec::new();

        f.read_to_end(&mut buffer)
            .or_else(|e| Err(ReadMacaroonError::ReadFileFail(e)))?;
        Ok(Macaroon(buffer))
    }
}
