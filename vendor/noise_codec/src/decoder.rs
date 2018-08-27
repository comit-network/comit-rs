use bytes::BytesMut;
use error::Error;
use payload_size::PayloadSize;
use tokio_codec::Decoder;
use NoiseCodec;
use LENGTH_FRAME_LENGTH;
use NOISE_TAG_LENGTH;

impl<C: Decoder> Decoder for NoiseCodec<C> {
    type Item = C::Item;
    type Error = Error<C::Error>;

    fn decode(&mut self, cipher_text: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let available_bytes = cipher_text.len();
        let enough_data_for_length = available_bytes >= LENGTH_FRAME_LENGTH;

        match self.payload_size {
            None if enough_data_for_length => {
                let payload_frame_len: PayloadSize =
                    self.decrypt(cipher_text, LENGTH_FRAME_LENGTH)?;
                let payload_frame_len = payload_frame_len.into();

                self.payload_size = Some(payload_frame_len);

                debug!("Next payload is {} bytes long.", payload_frame_len);

                self.decode(cipher_text)
            }
            None => {
                debug!(
                    "Need {} bytes to decrypt payload length. Got {}.",
                    LENGTH_FRAME_LENGTH, available_bytes
                );

                return Ok(None);
            }
            Some(payload_size) if available_bytes < payload_size => {
                let missing_bytes = payload_size - available_bytes;

                debug!(
                    "Waiting for {} more bytes until payload of length {} can be decrypted.",
                    missing_bytes, payload_size
                );

                return Ok(None);
            }
            Some(payload_size) => {
                debug!("Decrypting payload with {} bytes.", payload_size);

                let payload: Vec<u8> = self.decrypt(cipher_text, payload_size)?;

                self.payload_buffer.extend_from_slice(&payload);
                self.payload_size = None;

                let item = self
                    .inner
                    .decode(&mut self.payload_buffer)
                    .map_err(Error::Inner)?;

                match item {
                    Some(item) => Ok(Some(item)),
                    None => {
                        debug!("Successfully decrypted payload but target item did not fit into one payload. Waiting for more data.");
                        self.decode(cipher_text)
                    }
                }
            }
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

        let mut cleartext = vec![0; cleartext_length];

        if cipher_text.len() > number_of_bytes_to_decrypt {
            let cipher_text = cipher_text.split_to(number_of_bytes_to_decrypt);
            self.noise.read_message(&cipher_text[..], &mut cleartext)?;
        } else {
            self.noise.read_message(&cipher_text[..], &mut cleartext)?;
        }

        Ok(cleartext.into())
    }
}
