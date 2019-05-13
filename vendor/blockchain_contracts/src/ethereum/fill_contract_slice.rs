use byteorder::{BigEndian, ByteOrder};
use web3::types::{Address, U256};

pub trait FillContractSlice {
    fn fill_contract_slice(&self, buf: &mut [u8]);
}

impl FillContractSlice for Address {
    fn fill_contract_slice(&self, buf: &mut [u8]) {
        buf.copy_from_slice(&self[..]);
    }
}

impl FillContractSlice for U256 {
    fn fill_contract_slice(&self, buf: &mut [u8]) {
        self.to_big_endian(buf);
    }
}

impl FillContractSlice for u32 {
    fn fill_contract_slice(&self, buf: &mut [u8]) {
        BigEndian::write_u32(buf, *self);
    }
}

impl FillContractSlice for SecretHash {
    fn fill_contract_slice(&self, buf: &mut [u8]) {
        buf.copy_from_slice(&self.0[..]);
    }
}

#[derive(Debug)]
pub struct SecretHash(pub [u8; 32]);
