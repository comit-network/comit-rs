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

impl AsMut<[u8]> for Length {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

struct LengthFrame([u8; LENGTH_FRAME_LENGTH]);

impl LengthFrame {
    fn new() -> Self {
        LengthFrame([0u8; LENGTH_FRAME_LENGTH])
    }
}

struct PayloadFrame(Vec<u8>);

impl PayloadFrame {
    fn new(payload_size: usize) -> Self {
        PayloadFrame(vec![0; payload_size + NOISE_TAG_LENGTH])
    }
}

impl AsMut<[u8]> for LengthFrame {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl AsMut<[u8]> for PayloadFrame {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

impl AsRef<[u8]> for LengthFrame {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for PayloadFrame {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

struct EncodingItemBuffer {
    item_bytes: BytesMut,
    next_payload_length: usize,
}

impl EncodingItemBuffer {
    fn new(bytes: BytesMut) -> Self {
        EncodingItemBuffer {
            item_bytes: bytes,
            next_payload_length: 0,
        }
    }

    fn finished_encoding(&self) -> bool {
        self.item_bytes.is_empty()
    }

    fn compute_next_payload_length(&mut self) {
        self.next_payload_length = min(self.item_bytes.len(), MAX_PAYLOAD_LENGTH);

        debug!("Next payload length is {}", self.next_payload_length);
    }

    fn total_size(&self) -> usize {
        LENGTH_FRAME_LENGTH + self.next_payload_length + NOISE_TAG_LENGTH
    }

    fn encode_length(&mut self, noise: &mut Session) -> Result<LengthFrame, snow::SnowError> {
        let length = Length::new(self.next_payload_length);
        let mut length_frame = LengthFrame::new();

        trace!("Length: {:?}", length.as_ref());

        noise.write_message(length.as_ref(), length_frame.as_mut())?;

        trace!("Length-Frame: {:?}", length_frame.as_ref());

        Ok(length_frame)
    }

    fn encode_payload(&mut self, noise: &mut Session) -> Result<PayloadFrame, snow::SnowError> {
        let payload = self.item_bytes.split_to(self.next_payload_length);
        let mut payload_frame = PayloadFrame::new(self.next_payload_length);

        trace!("Payload: {:?}", payload.as_ref());

        noise.write_message(&payload[..], payload_frame.as_mut())?;

        trace!("Payload-Frame: {:?}", payload_frame.as_ref());

        Ok(payload_frame)
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

        let mut item_buffer = EncodingItemBuffer::new(item_bytes);

        while !item_buffer.finished_encoding() {
            item_buffer.compute_next_payload_length();
            let length_frame = item_buffer.encode_length(&mut self.noise)?;
            let payload_frame = item_buffer.encode_payload(&mut self.noise)?;

            cipher_text.reserve(item_buffer.total_size());
            cipher_text.put(length_frame.as_ref());
            cipher_text.put(payload_frame.as_ref());
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
