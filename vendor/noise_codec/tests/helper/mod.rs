use bytes::{BufMut, Bytes, BytesMut};
use noise_codec::NoiseCodec;
use snow;
use std::io;
use tokio_codec::{Decoder, Encoder};

pub fn init_noise<C: Encoder + Decoder + Clone>(codec: C) -> (NoiseCodec<C>, NoiseCodec<C>) {
    let mut noise_1 = snow::Builder::new("Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap())
        .build_initiator()
        .unwrap();

    let mut noise_2 = snow::Builder::new("Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap())
        .build_responder()
        .unwrap();

    let mut buf1 = [0u8; 65535];
    let mut buf2 = [0u8; 65535];
    // write first handshake message
    let len = noise_1.write_message(&[], &mut buf1).unwrap();
    let _len = noise_2.read_message(&buf1[..len], &mut buf2).unwrap();
    let len = noise_2.write_message(&[], &mut buf1).unwrap();
    let _len = noise_1.read_message(&buf1[..len], &mut buf2).unwrap();

    let noise_1 = noise_1.into_transport_mode().unwrap();
    let noise_2 = noise_2.into_transport_mode().unwrap();

    (
        NoiseCodec::new(noise_1, codec.clone()),
        NoiseCodec::new(noise_2, codec),
    )
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BytesCodec(());

impl BytesCodec {
    /// Creates a new `BytesCodec` for shipping around raw bytes.
    pub fn new() -> BytesCodec {
        BytesCodec(())
    }
}

impl Decoder for BytesCodec {
    type Item = Bytes;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Bytes>, io::Error> {
        if buf.len() > 0 {
            let len = buf.len();
            Ok(Some(buf.split_to(len).into()))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for BytesCodec {
    type Item = Bytes;
    type Error = io::Error;

    fn encode(&mut self, data: Bytes, buf: &mut BytesMut) -> Result<(), io::Error> {
        buf.reserve(data.len());
        buf.put(data);
        Ok(())
    }
}

pub fn msg(bytes: &[u8]) -> Bytes {
    Bytes::from(bytes.to_vec())
}
