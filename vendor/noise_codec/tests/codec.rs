extern crate bytes;
extern crate env_logger;
extern crate noise_codec;
extern crate snow;
extern crate spectral;
extern crate tokio_codec;

use spectral::prelude::*;

mod helper;

use bytes::BytesMut;
use helper::*;
use tokio_codec::{Decoder, Encoder, LinesCodec};

#[test]
fn encode_and_decode_hello_world() {
    let _ = env_logger::try_init();

    let (mut alice, mut bob) = init_noise(BytesCodec::new());
    {
        let mut cipher_text = BytesMut::new();
        alice.encode(msg(b"hello world"), &mut cipher_text).unwrap();

        let actual_message = bob.decode(&mut cipher_text);

        assert_that(&actual_message)
            .is_ok()
            .is_some()
            .is_equal_to(&msg(b"hello world"));
    }

    {
        let mut cipher_text = BytesMut::new();
        alice
            .encode(msg(b"you are beautiful!!!"), &mut cipher_text)
            .unwrap();

        let actual_message = bob.decode(&mut cipher_text);

        assert_that(&actual_message)
            .is_ok()
            .is_some()
            .is_equal_to(&msg(b"you are beautiful!!!"));
    }
}

#[test]
fn encode_two_messages_and_decode() {
    let _ = env_logger::try_init();

    let (mut alice, mut bob) = init_noise(LinesCodec::new());
    {
        let mut cipher_text = BytesMut::new();
        alice
            .encode("hello world".to_string(), &mut cipher_text)
            .unwrap();
        alice
            .encode("you are beautiful!!!".to_string(), &mut cipher_text)
            .unwrap();
        let msg1 = bob.decode(&mut cipher_text);
        let msg2 = bob.decode(&mut cipher_text);

        assert_that(&msg1)
            .is_ok()
            .is_some()
            .is_equal_to(String::from("hello world"));
        assert_that(&msg2)
            .is_ok()
            .is_some()
            .is_equal_to(String::from("you are beautiful!!!"));
    }
}

#[test]
fn decode_partial_message() {
    let _ = env_logger::try_init();

    let (mut alice, mut bob) = init_noise(BytesCodec::new());
    {
        let mut cipher_text = BytesMut::new();

        let actual_message = bob.decode(&mut cipher_text);
        assert_that(&actual_message).is_ok().is_none();

        alice.encode(msg(b"0123456789"), &mut cipher_text).unwrap();

        let mut buf = cipher_text.split_to(6);
        let after_6_bytes = bob.decode(&mut buf);
        asserting("shouldn't be a full message yet")
            .that(&after_6_bytes)
            .is_ok()
            .is_none();

        buf.extend_from_slice(&cipher_text.split_to(11)[..]);
        let after_17_bytes = bob.decode(&mut buf);
        asserting("still shouldn't be a full message yet")
            .that(&after_17_bytes)
            .is_ok()
            .is_none();

        buf.extend_from_slice(&cipher_text.split_to(11)[..]);
        let after_28_bytes = bob.decode(&mut buf);
        asserting("given the message cipher text and MAC still shouldn't have a message")
            .that(&after_28_bytes)
            .is_ok()
            .is_none();

        buf.extend_from_slice(&cipher_text[..]);
        let after_all_bytes = bob.decode(&mut buf);
        assert_that(&after_all_bytes)
            .is_ok()
            .is_some()
            .is_equal_to(&msg(b"0123456789"));
    }
}

#[test]
fn decode_message_spanning_multiple_noise_frames() {
    let (mut alice, mut bob) = init_noise(LinesCodec::new());
    let message_1 = String::from_utf8(vec![b'X'; 70_000]).unwrap();
    let message_2 = String::from_utf8(vec![b'Y'; 70_000]).unwrap();

    let mut cipher_text = BytesMut::new();
    alice.encode(message_1.clone(), &mut cipher_text).unwrap();
    alice.encode(message_2.clone(), &mut cipher_text).unwrap();

    {
        let item = bob.decode(&mut cipher_text);
        assert_that(&item).is_ok().is_some().is_equal_to(&message_1);
    }

    {
        // The codec shouldn't be consuming more bytes than it needs
        // to produce one message
        let item = bob.decode(&mut BytesMut::new());
        assert_that(&item).is_ok().is_none();
    }

    {
        let item = bob.decode(&mut cipher_text);
        assert_that(&item).is_ok().is_some().is_equal_to(&message_2);
    }
}
