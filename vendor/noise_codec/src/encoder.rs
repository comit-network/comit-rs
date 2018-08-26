use bytes::{BufMut, BytesMut};
use std::cmp::min;
use tokio_codec::Encoder;
use Error;
use NoiseCodec;
use PayloadLength;
use MAX_PAYLOAD_LENGTH;
use NOISE_TAG_LENGTH;

trait Len {
    fn len(&self) -> usize;
}

impl Len for PayloadLength {
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

            let length = PayloadLength::new(next_payload_length);
            let length_frame = self.encrypt(length)?;
            cipher_text.reserve(2 + NOISE_TAG_LENGTH);
            cipher_text.put(length_frame);

            let payload = item_bytes.split_to(next_payload_length);
            let payload_frame = self.encrypt(payload)?;
            cipher_text.reserve(next_payload_length + NOISE_TAG_LENGTH);
            cipher_text.put(payload_frame);
        }

        Ok(())
    }
}
