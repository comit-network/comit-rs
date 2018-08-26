extern crate byteorder;
extern crate bytes;
extern crate futures;
extern crate snow;
extern crate tokio;
extern crate tokio_codec;
#[macro_use]
extern crate log;

use byteorder::{BigEndian, ByteOrder};
use bytes::BytesMut;
use snow::Session;
use std::io;

mod decoder;
mod encoder;

/*
This is the format for each message. It first contains
a noise encrypted length (and tag/MAC) and then a noise
encrypted payload.
+-------------------------------+
|2-byte encrypted message length|
+-------------------------------+
|  16-byte tag of the encrypted |
|        message length         |
+-------------------------------+
|                               |
|                               |
|     encrypted payload         |
|     (max 65519 bytes)         |
|                               |
+-------------------------------+
|     16-byte tag of the        |
|      payload                  |
+-------------------------------+
*/

pub const NOISE_MSG_MAX_LENGTH: usize = 65535;
pub const NOISE_TAG_LENGTH: usize = 16;

pub const LENGTH_FRAME_LENGTH: usize = NOISE_TAG_LENGTH + 2;
pub const MAX_PAYLOAD_LENGTH: usize = NOISE_MSG_MAX_LENGTH - NOISE_TAG_LENGTH;

pub struct NoiseCodec<C> {
    noise: Session,
    inner: C,
    payload_frame_len: Option<usize>,
    payload_buffer: BytesMut,
}

impl<C> NoiseCodec<C> {
    pub fn new(noise: Session, inner: C) -> Self {
        NoiseCodec {
            noise,
            inner,
            payload_frame_len: None,
            payload_buffer: BytesMut::new(),
        }
    }
}

#[derive(Debug)]
pub enum Error<E> {
    IoError(io::Error),
    SnowError(snow::SnowError),
    InnerError(E),
}

impl<E> From<io::Error> for Error<E> {
    fn from(e: io::Error) -> Error<E> {
        Error::IoError(e)
    }
}

impl<E> From<snow::SnowError> for Error<E> {
    fn from(e: snow::SnowError) -> Error<E> {
        Error::SnowError(e)
    }
}

#[derive(Debug)]
struct PayloadLength([u8; 2]);

impl PayloadLength {
    fn new(length: usize) -> Self {
        let mut data = [0u8; 2];

        let total_length = length + NOISE_TAG_LENGTH;

        BigEndian::write_u16(&mut data, total_length as u16);

        PayloadLength(data)
    }

    fn as_usize(&self) -> usize {
        BigEndian::read_u16(&self.0[..]) as usize
    }
}

impl From<Vec<u8>> for PayloadLength {
    fn from(vec: Vec<u8>) -> Self {
        PayloadLength([vec[0], vec[1]])
    }
}

impl AsRef<[u8]> for PayloadLength {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}
