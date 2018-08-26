use bytes::BytesMut;
use tokio_codec::Decoder;
use Error;
use NoiseCodec;
use PayloadLength;
use LENGTH_FRAME_LENGTH;
use NOISE_TAG_LENGTH;

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
            let length: PayloadLength = self.decrypt(cipher_text, LENGTH_FRAME_LENGTH)?;

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
