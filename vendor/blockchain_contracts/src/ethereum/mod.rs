use crate::rfc003::{secret_hash::SecretHash, timestamp::Timestamp};
use byteorder::{BigEndian, WriteBytesExt};
use web3::types::{Address, U256};

pub mod rfc003;

#[derive(Debug)]
pub enum ToEvmError {
    Io(std::io::Error),
}

pub trait EncodeToEvm {
    fn encode_to_evm(&self) -> Result<Vec<u8>, ToEvmError>;
}

impl EncodeToEvm for SecretHash {
    fn encode_to_evm(&self) -> Result<Vec<u8>, ToEvmError> {
        Ok(self.clone().into())
    }
}

impl EncodeToEvm for Address {
    fn encode_to_evm(&self) -> Result<Vec<u8>, ToEvmError> {
        Ok(self.to_vec())
    }
}

impl EncodeToEvm for U256 {
    fn encode_to_evm(&self) -> Result<Vec<u8>, ToEvmError> {
        let mut vec = vec![0; 32];
        self.to_big_endian(&mut vec);
        Ok(vec)
    }
}

impl EncodeToEvm for Timestamp {
    fn encode_to_evm(&self) -> Result<Vec<u8>, ToEvmError> {
        let mut vec = vec![];
        vec.write_u32::<BigEndian>(self.clone().into())
            .map_err(ToEvmError::Io)?;
        Ok(vec)
    }
}
