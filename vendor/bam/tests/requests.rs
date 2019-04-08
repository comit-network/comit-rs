#[macro_use]
extern crate serde_derive;

#[macro_use]
pub mod common;
use crate::common::{
    setup::{create_server_with, start_server_with},
    *,
};
use bam::{connection, json};
use futures::future::Future;
use spectral::prelude::*;

#[test]
fn ping_message() {
    let (_runtime, alice, _bob) = start_server_with(ping::config());

    let actual_response_from_bob = alice
        .send_with_newline(r#"{"type":"REQUEST","id":10,"payload":{"type":"PING"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":10,"payload":{"status":"OK00"}}"#.into());
}

#[test]
fn two_ping_messages() {
    let (_runtime, alice, _bob) = start_server_with(ping::config());

    let response1 = alice
        .send_with_newline(r#"{"type":"REQUEST","id":10,"payload":{"type":"PING"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    let response2 = alice
        .send_with_newline(r#"{"type":"REQUEST","id":11,"payload":{"type":"PING"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&response1)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":10,"payload":{"status":"OK00"}}"#.into());

    assert_that(&response2)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":11,"payload":{"status":"OK00"}}"#.into());
}

#[test]
fn handle_invalid_json_line() {
    let (alice, bob_server) = create_server_with(ping::config());

    let bob_shutdown = bob_server
        .join(
            alice
                .send_with_newline(r#"{"type":"REQUEST","id":10,"payload":{"type":"PING""#)
                .map_err(|_e| {
                    unreachable!("Cannot happen because the shutdown handle sender shouldn't be able to go out of scope")
                }),
        )
        .and_then(|_| Ok(()))
        .or_else(|bob_error| Err(bob_error))
        .wait();

    match bob_shutdown {
        Ok(_) => panic!("should have shutdown with error"),
        Err(connection::ClosedReason::CodecError(json::Error::Json(_))) => {}
        _ => panic!("Expected a Json error"),
    }
}

#[test]
fn handle_unknown_frame_type() {
    let (alice, bob_server) = create_server_with(ping::config());

    let bob_shutdown = bob_server
        .join(
            alice
                .send_with_newline(r#"{"type":"I_DONT_EXIST","id":10,"payload":{"type":"PING"}}"#)
                .map_err(|_e| unreachable!()),
        )
        .and_then(|_| Ok(()))
        .or_else(|bob_error| Err(bob_error))
        .wait();

    match bob_shutdown {
        Ok(_) => panic!("should have shutdown with error"),
        Err(connection::ClosedReason::InvalidFrame(bam::Error::UnknownFrameType(
            ref frame_type,
        ))) => assert_eq!(frame_type, "I_DONT_EXIST"),
        _ => panic!("Expected an UnknownFrameType error"),
    }
}

#[test]
fn unknown_message() {
    let (_runtime, alice, _bob) = start_server_with(ping::config());
    let actual_response_from_bob = alice
        .send_with_newline(r#"{"type":"REQUEST","id":10,"payload":{"type":"UNKNOWN"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":10,"payload":{"status":"SE02"}}"#.into());
}

#[test]
fn reject_out_of_order_request() {
    let (mut _runtime, alice, _bob) = start_server_with(ping::config());

    let response1 = alice
        .send_with_newline(r#"{"type":"REQUEST","id":10,"payload":{"type":"PING"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    let out_of_error_request = alice
        .send_with_newline(r#"{"type":"REQUEST","id":9,"payload":{"type":"PING"}}"#)
        .wait();

    let response2 = alice
        .send_with_newline(r#"{"type":"REQUEST","id":11,"payload":{"type":"PING"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&response1)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":10,"payload":{"status":"OK00"}}"#.into());

    assert_that(&out_of_error_request).is_ok();

    assert_that(&response2)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":11,"payload":{"status":"OK00"}}"#.into());
}

#[test]
fn request_and_response_with_string_headers() {
    let (mut _runtime, alice, _bob) = start_server_with(say_hello::config());

    let actual_response_from_bob = alice
        .send_with_newline(include_json_line!("say_hello_to_world_request.json"))
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!(
            "say_hello_to_world_response_compact_id-10.json"
        ));
}

#[test]
fn request_and_response_with_compact_string_headers() {
    let (mut _runtime, alice, _bob) = start_server_with(say_hello::config());

    let actual_response_from_bob = alice
        .send_with_newline(include_json_line!(
            "say_hello_to_world_compact_header_request.json"
        ))
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!(
            "say_hello_to_world_response_compact_id-10.json"
        ));
}

#[test]
fn unknown_non_mandatory_header_gets_ignored() {
    let (mut _runtime, alice, _bob) = start_server_with(ping::config());

    let actual_response_from_bob = alice
        .send_with_newline(include_json_line!("ping_with_non_mandatory_header.json"))
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!("empty_response_id-10_ok00.json"));
}

#[test]
fn rejects_malformed_header_without_value() {
    let (mut _runtime, alice, _bob) = start_server_with(ping::config());

    let actual_response_from_bob = alice
        .send_with_newline(include_json_line!("ping_with_malformed_header.json"))
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!("empty_response_se00.json"));
}

#[test]
fn can_parse_json_integer_value_in_header() {
    let (mut _runtime, alice, _bob) = start_server_with(say_hello::config());

    let actual_response_from_bob = alice
        .send_with_newline(include_json_line!(
            "say_hello_to_world_2_times_request.json"
        ))
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!(
            "say_hello_to_world_2_times_response.json"
        ));
}

#[test]
fn can_parse_header_parameters() {
    let (mut _runtime, alice, _bob) = start_server_with(buy::config());

    let buy_phone_response = alice
        .send_with_newline(include_json_line!("buy_phone_request.json"))
        .and_then(|_| alice.receive());

    let buy_retro_encabulator_response = alice
        .send_with_newline(include_json_line!("buy_retro_encabulator_request.json"))
        .and_then(|_| alice.receive());

    assert_that(&buy_phone_response.wait())
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!("buy_phone_response.json"));

    assert_that(&buy_retro_encabulator_response.wait())
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!("buy_retro_encabulator_response.json"));
}

#[test]
fn unknown_mandatory_header_triggers_error_response() {
    let (mut _runtime, alice, _bob) = start_server_with(say_hello::config());

    let actual_response_from_bob = alice
        .send_with_newline(include_json_line!(
            "say_hello_with_unknown_mandatory_header.json"
        ))
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!("unsupported_lang_header_response.json"));
}

#[test]
fn handle_malformed_request_type() {
    let (mut _runtime, alice, _bob) = start_server_with(ping::config());

    let actual_response_from_bob = alice
        .send_with_newline(include_json_line!("malformed_request_type.json"))
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!("empty_response_se00.json"));
}

#[test]
fn handle_malformed_headers() {
    let (mut _runtime, alice, _bob) = start_server_with(say_hello::config());

    let actual_response_from_bob = alice
        .send_with_newline(include_json_line!(
            "say_hello_to_world_malformed_header_request.json"
        ))
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!("empty_response_se00.json"));
}

#[test]
fn handle_request_with_payload() {
    let (mut _runtime, alice, _bob) = start_server_with(place_order::config());

    let android_order_response = alice
        .send_with_newline(include_json_line!("place_android_phone_order_request.json"))
        .and_then(|_| alice.receive())
        .wait();

    let apple_order_response = alice
        .send_with_newline(include_json_line!("place_apple_phone_order_request.json"))
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&android_order_response)
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!(
            "place_android_phone_order_response.json"
        ));

    assert_that(&apple_order_response)
        .is_ok()
        .is_some()
        .is_equal_to(include_json_line!("place_apple_phone_order_response.json"));
}

#[test]
fn handle_request_with_mutable_state() {
    let (mut _runtime, alice, _bob) = start_server_with(counter::config());

    let count_1 = alice
        .send_with_newline(r#"{"type":"REQUEST","id":0,"payload":{"type":"COUNT"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    let count_2 = alice
        .send_with_newline(r#"{"type":"REQUEST","id":1,"payload":{"type":"COUNT"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&count_1)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":0,"payload":{"body":1,"status":"OK00"}}"#.into());

    assert_that(&count_2)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":1,"payload":{"body":2,"status":"OK00"}}"#.into());
}
