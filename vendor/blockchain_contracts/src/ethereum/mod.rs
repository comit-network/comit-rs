use byteorder::{BigEndian, ByteOrder};
use web3::types::{Address, U256};

pub mod rfc003;

pub trait EncodeToEvm {
    fn encode_to_evm(&self) -> Vec<u8>;
}

impl EncodeToEvm for &[u8] {
    fn encode_to_evm(&self) -> Vec<u8> {
        self.to_vec()
    }
}

impl EncodeToEvm for Address {
    fn encode_to_evm(&self) -> Vec<u8> {
        self.to_vec()
    }
}

impl EncodeToEvm for U256 {
    fn encode_to_evm(&self) -> Vec<u8> {
        let mut vec = vec![0; 32];
        self.to_big_endian(&mut vec);
        vec
    }
}

impl EncodeToEvm for u32 {
    fn encode_to_evm(&self) -> Vec<u8> {
        let mut buf = [0; 4];
        BigEndian::write_u32(&mut buf, *self);
        buf.to_vec()
    }
}
