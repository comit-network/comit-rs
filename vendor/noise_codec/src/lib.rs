extern crate byteorder;
extern crate bytes;
extern crate futures;
extern crate snow;
extern crate tokio;
extern crate tokio_codec;

use byteorder::{BigEndian, ByteOrder};
use bytes::{BufMut, BytesMut};
use snow::Session;
use std::{cmp::min, io};
use tokio_codec::{Decoder, Encoder};

pub const NOISE_MAXMSGLEN: usize = 65535;
pub const NOISE_TAGLEN: usize = 16;
pub const LENGTH_FRAME_LEN: usize = NOISE_TAGLEN + 2;
pub const MAX_PAYLOAD_LEN: usize = NOISE_MAXMSGLEN - NOISE_TAGLEN;

pub struct NoiseCodec<C> {
    noise: Session,
    inner: C,
    payload_frame_len: Option<usize>,
    payload_buffer: BytesMut,
}

impl<C: Encoder + Decoder> NoiseCodec<C> {
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

impl<C: Encoder> Encoder for NoiseCodec<C> {
    type Item = C::Item;
    type Error = Error<C::Error>;

    fn encode(&mut self, item: C::Item, encrypted: &mut BytesMut) -> Result<(), Self::Error> {
        let mut item_bytes = BytesMut::new();
        self.inner
            .encode(item, &mut item_bytes)
            .map_err(Error::InnerError)?;

        while !item_bytes.is_empty() {
            let total_len = item_bytes.len();
            let payload_len = min(total_len, MAX_PAYLOAD_LEN);
            let payload = item_bytes.split_to(payload_len);
            let payload_frame_len = payload.len() + NOISE_TAGLEN;
            encrypted.reserve(payload_frame_len + LENGTH_FRAME_LEN);
            {
                let mut be_length = [0u8; 2];
                BigEndian::write_u16(&mut be_length, payload_frame_len as u16);
                let mut length_frame = [0u8; LENGTH_FRAME_LEN];
                self.noise.write_message(&be_length[..], &mut length_frame)?;
                encrypted.put(&length_frame[..]);
            }

            {
                let mut payload_frame = vec![0u8; payload_frame_len];
                self.noise.write_message(&payload[..], &mut payload_frame)?;
                encrypted.put(payload_frame);
            }
        }

        Ok(())
    }
}

impl<C: Decoder> Decoder for NoiseCodec<C> {
    type Item = C::Item;
    type Error = Error<C::Error>;

    fn decode(&mut self, cipher_text: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            let payload_frame_len = match self.payload_frame_len {
                None => {
                    if cipher_text.len() < LENGTH_FRAME_LEN {
                        break;
                    }

                    let payload_frame_len = {
                        let length_frame = cipher_text.split_to(LENGTH_FRAME_LEN);
                        let mut payload_frame_len = [0u8; 2];
                        self.noise
                            .read_message(&length_frame[..], &mut payload_frame_len)?;
                        BigEndian::read_u16(&payload_frame_len) as usize
                    };

                    self.payload_frame_len = Some(payload_frame_len);
                    payload_frame_len
                }
                Some(payload_frame_len) => payload_frame_len,
            };

            if cipher_text.len() < payload_frame_len {
                break;
            }

            let payload_len = payload_frame_len - NOISE_TAGLEN;
            let mut payload = vec![0u8; payload_len];
            self.noise
                .read_message(&cipher_text[..payload_frame_len], &mut payload[..])?;

            self.payload_buffer.extend_from_slice(&payload);
            cipher_text.advance(payload_frame_len);
            self.payload_frame_len = None;

            let item = self
                .inner
                .decode(&mut self.payload_buffer)
                .map_err(Error::InnerError)?;

            if item.is_some() {
                return Ok(item);
            }
        }
        Ok(None)
    }
}
