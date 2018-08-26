use byteorder::{BigEndian, ByteOrder};
use NOISE_TAG_LENGTH;

#[derive(Debug)]
pub struct PayloadSize([u8; 2]);

impl From<PayloadSize> for usize {
    fn from(payload_length: PayloadSize) -> Self {
        BigEndian::read_u16(&payload_length.0[..]) as usize
    }
}

impl From<usize> for PayloadSize {
    fn from(length: usize) -> Self {
        let mut data = [0u8; 2];

        let total_length = length + NOISE_TAG_LENGTH;

        BigEndian::write_u16(&mut data, total_length as u16);

        PayloadSize(data)
    }
}

impl From<Vec<u8>> for PayloadSize {
    fn from(vec: Vec<u8>) -> Self {
        PayloadSize([vec[0], vec[1]])
    }
}

impl AsRef<[u8]> for PayloadSize {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}
