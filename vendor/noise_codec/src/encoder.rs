use bytes::{BufMut, BytesMut};
use std::cmp::min;
use tokio_codec::Encoder;

use error::Error;
use payload_size::PayloadSize;
use NoiseCodec;
use MAX_PAYLOAD_LENGTH;
use NOISE_TAG_LENGTH;

impl<C: Encoder> Encoder for NoiseCodec<C> {
    type Item = C::Item;
    type Error = Error<C::Error>;

    fn encode(&mut self, item: C::Item, cipher_text: &mut BytesMut) -> Result<(), Self::Error> {
        let mut item_bytes = BytesMut::new();

        self.inner
            .encode(item, &mut item_bytes)
            .map_err(Error::Inner)?;

        debug!(
            "Encoding {} bytes into {} frame(s).",
            item_bytes.len(),
            item_bytes.len() / MAX_PAYLOAD_LENGTH + 1
        );

        while !item_bytes.is_empty() {
            let payload_size = min(item_bytes.len(), MAX_PAYLOAD_LENGTH);

            let size_frame = self.encrypt(PayloadSize::from(payload_size))?;
            cipher_text.reserve(2 + NOISE_TAG_LENGTH);
            cipher_text.put(size_frame);

            let payload = item_bytes.split_to(payload_size);
            let payload_frame = self.encrypt(payload)?;
            cipher_text.reserve(payload_size + NOISE_TAG_LENGTH);
            cipher_text.put(payload_frame);
        }

        Ok(())
    }
}

trait Len {
    fn len(&self) -> usize;
}

impl Len for PayloadSize {
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
    fn encrypt<S: AsRef<[u8]> + Len>(&mut self, clear_text: S) -> Result<Vec<u8>, Error<C::Error>> {
        let cipher_text_length = clear_text.len() + NOISE_TAG_LENGTH;
        let mut cipher_text = vec![0u8; cipher_text_length];

        self.noise
            .write_message(clear_text.as_ref(), &mut cipher_text[..])?;

        Ok(cipher_text)
    }
}
