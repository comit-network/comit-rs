use crate::rfc003::{secret_hash::SecretHash, timestamp::Timestamp};
use binary_macros::{base16, base16_impl};
use byteorder::{BigEndian, ByteOrder};
use web3::types::{Address, Bytes, U256};

pub mod rfc003;

pub trait EncodeToEvm {
    fn encode_to_evm(&self) -> Vec<u8>;
}

impl EncodeToEvm for SecretHash {
    fn encode_to_evm(&self) -> Vec<u8> {
        self.clone().into()
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

impl EncodeToEvm for Timestamp {
    fn encode_to_evm(&self) -> Vec<u8> {
        let mut buf = [0; 4];
        BigEndian::write_u32(&mut buf, self.clone().into());
        buf.to_vec()
    }
}

/// Constructs the payload to transfer `Erc20` tokens to a `to_address`
pub fn transfer_erc20_tx_payload(token_quantity: U256, to_address: Address) -> Bytes {
    let transfer_fn_abi = base16!("A9059CBB");
    let to_address = <[u8; 20]>::from(to_address);
    let amount = <[u8; 32]>::from(token_quantity);

    let mut data = [0u8; 4 + 32 + 32];
    data[..4].copy_from_slice(transfer_fn_abi);
    data[16..36].copy_from_slice(&to_address);
    data[36..68].copy_from_slice(&amount);

    Bytes::from(data.to_vec())
}
