extern crate byteorder;
extern crate bytes;
extern crate futures;
extern crate snow;
extern crate tokio;
extern crate tokio_codec;
#[macro_use]
extern crate log;

use byteorder::{BigEndian, ByteOrder};
use bytes::{BufMut, BytesMut};
use snow::Session;
use std::{cmp::min, io};
use tokio_codec::{Decoder, Encoder};

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

trait Len {
    fn len(&self) -> usize;
}

impl Len for Length {
    fn len(&self) -> usize {
        2
    }
}

impl Len for BytesMut {
    fn len(&self) -> usize {
        self.len()
    }
}

impl<C: Encoder> NoiseCodec<C> {
    fn encrypt<S: AsRef<[u8]> + Len>(&mut self, cleartext: S) -> Result<Vec<u8>, Error<C::Error>> {
        let cipher_text_length = cleartext.len() + NOISE_TAG_LENGTH;

        let mut cipher_text = vec![0u8; cipher_text_length];

        self.noise
            .write_message(cleartext.as_ref(), &mut cipher_text[..])?;

        Ok(cipher_text)
    }
}

impl<C: Decoder> NoiseCodec<C> {
    fn decrypt<T: From<Vec<u8>>>(
        &mut self,
        cipher_text: &mut BytesMut,
        number_of_bytes_to_decrypt: usize,
    ) -> Result<T, Error<C::Error>> {
        let cleartext_length = number_of_bytes_to_decrypt - NOISE_TAG_LENGTH;
        debug!("Decrypting {} bytes", number_of_bytes_to_decrypt);

        let mut cleartext = vec![0; cleartext_length];

        if cipher_text.len() > number_of_bytes_to_decrypt {
            let cipher_text = cipher_text.split_to(number_of_bytes_to_decrypt);
            self.noise.read_message(&cipher_text[..], &mut cleartext)?;
        } else {
            self.noise.read_message(&cipher_text[..], &mut cleartext)?;
        }

        Ok(cleartext.into())
    }

    fn decode_payload_frame_length(
        &mut self,
        cipher_text: &mut BytesMut,
    ) -> Result<Option<usize>, Error<C::Error>> {
        let no_length_yet = self.payload_frame_len.is_none();
        let enough_data_for_length = cipher_text.len() >= LENGTH_FRAME_LENGTH;

        if no_length_yet && enough_data_for_length {
            let length: Length = self.decrypt(cipher_text, LENGTH_FRAME_LENGTH)?;

            let length = length.as_usize();
            debug!("Decrypted length: {:?}", length);

            self.payload_frame_len = Some(length);
        }

        Ok(self.payload_frame_len)
    }

    fn decode_payload_frame(
        &mut self,
        cipher_text: &mut BytesMut,
        payload_frame_len: usize,
    ) -> Result<Option<C::Item>, Error<C::Error>> {
        let payload: Vec<u8> = self.decrypt(cipher_text, payload_frame_len)?;

        self.payload_buffer.extend_from_slice(&payload);
        self.payload_frame_len = None;

        let item = self
            .inner
            .decode(&mut self.payload_buffer)
            .map_err(Error::InnerError)?;

        Ok(item)
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
struct Length([u8; 2]);

impl Length {
    fn new(length: usize) -> Self {
        let mut data = [0u8; 2];

        let total_length = length + NOISE_TAG_LENGTH;

        BigEndian::write_u16(&mut data, total_length as u16);

        Length(data)
    }

    fn as_usize(&self) -> usize {
        BigEndian::read_u16(&self.0[..]) as usize
    }
}

impl From<Vec<u8>> for Length {
    fn from(vec: Vec<u8>) -> Self {
        Length([vec[0], vec[1]])
    }
}

impl AsRef<[u8]> for Length {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl<C: Encoder> Encoder for NoiseCodec<C> {
    type Item = C::Item;
    type Error = Error<C::Error>;

    fn encode(&mut self, item: C::Item, cipher_text: &mut BytesMut) -> Result<(), Self::Error> {
        let mut item_bytes = BytesMut::new();

        self.inner
            .encode(item, &mut item_bytes)
            .map_err(Error::InnerError)?;

        while !item_bytes.is_empty() {
            let next_payload_length = min(item_bytes.len(), MAX_PAYLOAD_LENGTH);

            let length_frame = self.encrypt(Length::new(next_payload_length))?;

            let payload = item_bytes.split_to(next_payload_length);
            let payload_frame = self.encrypt(payload)?;

            cipher_text.reserve(LENGTH_FRAME_LENGTH + next_payload_length + NOISE_TAG_LENGTH);
            cipher_text.put(length_frame);
            cipher_text.put(payload_frame);
        }

        Ok(())
    }
}

impl<C: Decoder> Decoder for NoiseCodec<C> {
    type Item = C::Item;
    type Error = Error<C::Error>;

    fn decode(&mut self, cipher_text: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.decode_payload_frame_length(cipher_text)? {
            Some(payload_frame_length) => {
                if cipher_text.len() < payload_frame_length {
                    return Ok(None);
                }

                let item = self.decode_payload_frame(cipher_text, payload_frame_length)?;

                if item.is_none() {
                    return self.decode(cipher_text);
                }

                Ok(item)
            }
            None => Ok(None),
        }
    }
}
