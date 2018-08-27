extern crate byteorder;
extern crate bytes;
extern crate futures;
extern crate snow;
extern crate tokio;
extern crate tokio_codec;
#[macro_use]
extern crate log;

use bytes::BytesMut;
use snow::Session;

mod decoder;
mod encoder;
mod error;
mod payload_size;

pub use error::*;

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
    payload_size: Option<usize>,
    payload_buffer: BytesMut,
}

impl<C> NoiseCodec<C> {
    pub fn new(noise: Session, inner: C) -> Self {
        NoiseCodec {
            noise,
            inner,
            payload_size: None,
            payload_buffer: BytesMut::new(),
        }
    }
}
