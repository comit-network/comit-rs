extern crate bytes;
extern crate env_logger;
extern crate noise_codec;
extern crate snow;
extern crate tokio_codec;
use bytes::{Bytes, BytesMut};
use noise_codec::NoiseCodec;
use tokio_codec::{BytesCodec, Decoder, Encoder, LinesCodec};

fn init_noise<C: Encoder + Decoder + Clone>(codec: C) -> (NoiseCodec<C>, NoiseCodec<C>) {
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

#[test]
fn encode_and_decode_hello_world() {
    let _ = env_logger::try_init();

    let (mut alice, mut bob) = init_noise(BytesCodec::new());
    {
        let bytes = Bytes::from(b"hello world".to_vec());
        let mut cipher_text = BytesMut::new();
        alice.encode(bytes, &mut cipher_text).unwrap();
        let msg = bob.decode(&mut cipher_text).unwrap().unwrap();
        assert_eq!(&msg[..], b"hello world");
    }

    {
        let bytes = Bytes::from(b"you are beautiful!!!".to_vec());
        let mut cipher_text = BytesMut::new();
        alice.encode(bytes, &mut cipher_text).unwrap();
        let msg = bob.decode(&mut cipher_text).unwrap().unwrap();
        assert_eq!(&msg[..], b"you are beautiful!!!");
    }
}

#[test]
fn encode_two_messages_and_decode() {
    let (mut alice, mut bob) = init_noise(LinesCodec::new());
    {
        let mut cipher_text = BytesMut::new();
        alice
            .encode("hello world".to_string(), &mut cipher_text)
            .unwrap();
        alice
            .encode("you are beautiful!!!".to_string(), &mut cipher_text)
            .unwrap();
        let msg1 = bob.decode(&mut cipher_text).unwrap();
        let msg2 = bob.decode(&mut cipher_text).unwrap();
        assert_eq!(msg1, Some(String::from("hello world")));
        assert_eq!(msg2, Some(String::from("you are beautiful!!!")));
    }
}

#[test]
fn decode_partial_message() {
    let _ = env_logger::try_init();

    let (mut alice, mut bob) = init_noise(BytesCodec::new());
    {
        let bytes = Bytes::from(b"0123456789".to_vec());
        let mut cipher_text = BytesMut::new();

        let empty_message = bob.decode(&mut cipher_text);
        assert!(empty_message.unwrap().is_none());

        alice.encode(bytes, &mut cipher_text).unwrap();

        let mut buf = cipher_text.split_to(6);
        let after_6_bytes = bob.decode(&mut buf).unwrap();
        assert!(after_6_bytes.is_none(), "shouldn't be a full message yet");

        buf.extend_from_slice(&cipher_text.split_to(11)[..]);
        let after_17_bytes = bob.decode(&mut buf).unwrap();
        assert!(
            after_17_bytes.is_none(),
            "still shouldn't be a full message yet"
        );

        buf.extend_from_slice(&cipher_text.split_to(11)[..]);
        let after_28_bytes = bob.decode(&mut buf).unwrap();
        assert!(
            after_28_bytes.is_none(),
            "given the message cipher text sans MAC still shouldn't have a message"
        );

        buf.extend_from_slice(&cipher_text[..]);
        let after_all_bytes = bob.decode(&mut buf).unwrap();
        assert_eq!(
            after_all_bytes,
            Some(BytesMut::from(b"0123456789" as &[u8]))
        );
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
        let item = bob.decode(&mut cipher_text).unwrap();
        assert_eq!(item, Some(message_1));
    }

    {
        // The codec shouldn't be consuming more bytes than it needs
        // to produce one message
        let item = bob.decode(&mut BytesMut::new()).unwrap();
        assert!(item.is_none());
    }

    {
        let item = bob.decode(&mut cipher_text).unwrap();
        assert_eq!(item, Some(message_2));
    }
}
