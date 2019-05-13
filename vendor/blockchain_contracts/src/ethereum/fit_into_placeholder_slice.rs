use byteorder::{BigEndian, ByteOrder};
use web3::types::{Address, U256};

pub trait FitIntoPlaceholderSlice {
    fn fit_into_placeholder_slice(self, buf: &mut [u8]);
}

impl FitIntoPlaceholderSlice for Address {
    fn fit_into_placeholder_slice(self, buf: &mut [u8]) {
        buf.copy_from_slice(&self[..]);
    }
}

impl FitIntoPlaceholderSlice for U256 {
    fn fit_into_placeholder_slice(self, buf: &mut [u8]) {
        self.to_big_endian(buf);
    }
}

impl FitIntoPlaceholderSlice for u32 {
    fn fit_into_placeholder_slice(self, buf: &mut [u8]) {
        BigEndian::write_u32(buf, self);
    }
}

impl FitIntoPlaceholderSlice for SecretHash {
    fn fit_into_placeholder_slice(self, buf: &mut [u8]) {
        buf.copy_from_slice(&self.0[..]);
    }
}

#[derive(Debug)]
pub struct SecretHash(pub [u8; 32]);
