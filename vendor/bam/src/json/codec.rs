use crate::json;
use bytes::BytesMut;
use std::io;
use tokio_codec::{Decoder, Encoder};

#[derive(Debug)]
pub enum Error {
    Json(serde_json::Error),
    IO(io::Error),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

#[derive(Debug)]
pub struct JsonFrameCodec;

impl Default for JsonFrameCodec {
    fn default() -> Self {
        Self {}
    }
}

impl Encoder for JsonFrameCodec {
    type Item = json::Frame;
    type Error = Error;

    fn encode(&mut self, item: json::Frame, dst: &mut BytesMut) -> Result<(), Error> {
        let mut bytes = serde_json::to_vec(&item)?;
        bytes.push(b'\n');

        dst.extend(bytes);

        Ok(())
    }
}

impl Decoder for JsonFrameCodec {
    type Item = json::Frame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<json::Frame>, Error> {
        match src.iter().position(|b| *b == b'\n') {
            Some(position) => {
                let frame_bytes = src.split_to(position + 1);
                let frame = serde_json::from_slice(frame_bytes.as_ref())?;
                Ok(Some(frame))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use spectral::prelude::*;

    #[test]
    fn should_encode_frame_to_bytes() {
        let frame = json::Frame::new("FOO".into(), 0, serde_json::Value::Null);

        let mut codec = JsonFrameCodec::default();

        let mut bytes = BytesMut::new();

        assert!(codec.encode(frame, &mut bytes).is_ok());

        let frame_bytes = br#"{"type":"FOO","id":0,"payload":null}"#.as_ref();
        let newline = b"\n".as_ref();

        let expected = [frame_bytes, newline].concat();

        assert_eq!(&bytes[..], &expected[..]);
    }

    #[test]
    fn should_decode_bytes_to_frame() {
        let frame_bytes = br#"{"type":"FOO","id":0,"payload":null}"#.as_ref();
        let newline = b"\n".as_ref();

        let mut codec = JsonFrameCodec::default();

        let mut bytes = BytesMut::new();
        bytes.extend([frame_bytes, newline].concat());

        let expected_frame = json::Frame::new("FOO".into(), 0, serde_json::Value::Null);

        assert_that(&codec.decode(&mut bytes))
            .is_ok()
            .is_some()
            .is_equal_to(&expected_frame);
    }

    #[test]
    fn given_not_enough_bytes_should_wait_for_more() {
        let frame_bytes = br#"{"type":"FOO","#.as_ref();
        let remaining_bytes = br#""id":0,"payload":null}"#.as_ref();

        let mut codec = JsonFrameCodec::default();

        let mut bytes = BytesMut::new();
        bytes.extend(frame_bytes);

        assert_that(&codec.decode(&mut bytes)).is_ok().is_none();

        bytes.extend(remaining_bytes);
        bytes.extend(b"\n");

        assert_that(&codec.decode(&mut bytes)).is_ok().is_some();
    }

    #[test]
    fn given_two_frames_in_a_row_should_decode_both() {
        let frame_bytes = br#"{"type":"FOO","id":0,"payload":null}"#.as_ref();
        let newline = b"\n".as_ref();

        let mut codec = JsonFrameCodec::default();

        let mut bytes = BytesMut::new();
        bytes.extend([frame_bytes, newline, frame_bytes, newline].concat());

        let first = codec.decode(&mut bytes);
        let second = codec.decode(&mut bytes);

        let expected_frame = json::Frame::new("FOO".into(), 0, serde_json::Value::Null);

        assert_that(&first)
            .is_ok()
            .is_some()
            .is_equal_to(&expected_frame);

        assert_that(&second)
            .is_ok()
            .is_some()
            .is_equal_to(&expected_frame);
    }
}
