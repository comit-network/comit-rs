extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate futures;
extern crate memsocket;
extern crate pretty_env_logger;
extern crate spectral;
extern crate tokio;
extern crate tokio_codec;
extern crate transport_protocol;

mod common;
use common::*;
use futures::future::Future;
use spectral::prelude::*;
use transport_protocol::{config::Config, json::*, Error, Status};

#[test]
fn handle_ping_request_frame() {
    let (_runtime, alice, _bob) =
        setup(Config::new().on_request("PING", &[], |_: Request| Response::new(Status::OK(0))));

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
fn handle_two_ping_request_frame() {
    let (_runtime, alice, _bob) =
        setup(Config::new().on_request("PING", &[], |_: Request| Response::new(Status::OK(0))));

    let actual_response_from_bob = alice
        .send_with_newline(r#"{"type":"REQUEST","id":10,"payload":{"type":"PING"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":10,"payload":{"status":"OK00"}}"#.into());

    let actual_response_from_bob = alice
        .send_with_newline(r#"{"type":"REQUEST","id":11,"payload":{"type":"PING"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":11,"payload":{"status":"OK00"}}"#.into());
}

#[test]
fn handle_unknown_request_frame() {
    let (_runtime, alice, _bob) = setup(Config::new());

    let actual_response_from_bob = alice
        .send_with_newline(r#"{"type":"REQUEST","id":10,"payload":{"type":"PING"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":10,"payload":{"status":"SE02"}}"#.into());
}

#[test]
fn reject_out_of_order_request() {
    let (mut _runtime, alice, _bob) =
        setup(Config::new().on_request("PING", &[], |_: Request| Response::new(Status::OK(0))));

    let actual_response_from_bob = alice
        .send_with_newline(r#"{"type":"REQUEST","id":10,"payload":{"type":"PING"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":10,"payload":{"status":"OK00"}}"#.into());

    alice
        .send_with_newline(r#"{"type":"REQUEST","id":9,"payload":{"type":"PING"}}"#)
        .wait();

    let actual_response_from_bob = alice
        .send_with_newline(r#"{"type":"REQUEST","id":11,"payload":{"type":"PING"}}"#)
        .and_then(|_| alice.receive())
        .wait();

    assert_that(&actual_response_from_bob)
        .is_ok()
        .is_some()
        .is_equal_to(&r#"{"type":"RESPONSE","id":11,"payload":{"status":"OK00"}}"#.into());;
}

#[test]
fn request_and_response_with_string_headers() {
    let (mut handler, _) = gen_frame_handler();
    assert_successful(
        &mut handler,
        r#"{"type":"REQUEST","id":10,"payload":{"type":"SAY_HELLO", "headers":{"TO": {"value":"WORLD"}}}}"#,
        Some(r#"{"type":"RESPONSE","id":10,"payload":{"headers":{"HELLO":"WORLD"},"status":"OK00"}}"#),
    );
}

#[test]
fn request_and_response_with_compact_string_headers() {
    let (mut handler, _) = gen_frame_handler();
    assert_successful(
        &mut handler,
        r#"{"type":"REQUEST","id":10,"payload":{"type":"SAY_HELLO", "headers":{"TO": "WORLD"}}}"#,
        Some(r#"{"type":"RESPONSE","id":10,"payload":{"headers":{"HELLO":"WORLD"},"status":"OK00"}}"#),
    );
}

#[test]
fn unknown_non_mandatory_header_gets_ignored() {
    let (mut handler, _) = gen_frame_handler();
    assert_successful(
        &mut handler,
        r#"{"type":"REQUEST","id":10,"payload":{"type":"FOO", "headers":{"_SOMETHING": {"value":"42"}}}}"#,
        Some(r#"{"type":"RESPONSE","id":10,"payload":{"status":"OK00"}}"#),
    );
}

#[test]
fn rejects_malformed_header_without_value() {
    let (mut handler, _) = gen_frame_handler();
    assert_successful(
        &mut handler,
        json!({"type":"REQUEST","id":10,"payload":{"type":"FOO", "headers":{"_SOMETHING": {"number":"42"}}}})
            .to_string()
            .as_str(),
        Some(
            json!({
                "type": "RESPONSE",
                "id": 10,
                "payload": {
                    "status": "SE00",
                }
            }).to_string()
                .as_str(),
        ),
    );
}

#[test]
fn can_parse_json_integer_value_in_header() {
    let (mut handler, _) = gen_frame_handler();
    assert_successful(
        &mut handler,
        r#"{"type":"REQUEST","id":10,"payload":{"type":"SAY_HELLO", "headers":{"TO": "WORLD", "_TIMES": 2}}}"#,
        Some(r#"{"type":"RESPONSE","id":10,"payload":{"headers":{"HELLO":"WORLD WORLD"},"status":"OK00"}}"#),
    );
}

#[test]
fn can_parse_header_parameters() {
    let (mut handler, _) = gen_frame_handler();

    let input = json!({
      "type": "REQUEST",
      "id": 10,
      "payload": {
        "type": "BUY",
        "headers": {
          "THING": {
            "value": "PHONE",
            "parameters": {
                "os": "Android",
                "model": "Pixel 2 XL",
                "brand": "LG",
            }
          }
        }
      }
    });
    let expected_output = json!({
      "type": "RESPONSE",
      "id": 10,
      "payload": {
        "status": "OK00",
        "headers": {
          "PRICE": 420
        }
      }
    });

    assert_successful(
        &mut handler,
        input.to_string().as_str(),
        Some(expected_output.to_string().as_str()),
    );

    let input = json!({
      "type": "REQUEST",
      "id": 11,
      "payload": {
        "type": "BUY",
        "headers": {
          "THING": {
            "value": "RETRO ENCABULATOR",
          }
        }
      }
    });
    let expected_output = json!({
      "type": "RESPONSE",
      "id": 11,
      "payload": {
        "status": "OK00",
        "headers": {
          "PRICE": 9001
        }
      }
    });

    assert_successful(
        &mut handler,
        input.to_string().as_str(),
        Some(expected_output.to_string().as_str()),
    );
}

#[test]
fn unknown_mandatory_header_triggers_error_response() {
    let (mut handler, _) = gen_frame_handler();

    let input = json!({
      "type": "REQUEST",
      "id": 10,
      "payload": {
        "type": "SAY_HELLO",
        "headers": {
          "LANG": {
            "value": "ENG"
          },
          "TO": {
            "value": "WORLD"
          }
        }
      }
    });
    let expected_output = json!({
      "type": "RESPONSE",
      "id": 10,
      "payload": {
        "status": "SE01",
        "headers": {
          "Unsupported-Headers": ["LANG"]
        }
      }
    });

    assert_successful(
        &mut handler,
        input.to_string().as_str(),
        Some(expected_output.to_string().as_str()),
    );
}

#[test]
fn unknown_mandatory_header_defined_in_other_header_triggers_error_response() {
    let (mut handler, _) = gen_frame_handler();

    let input = json!({
        "type": "REQUEST",
        "id": 10,
        "payload": {
            "type": "SAY_HELLO",
            "headers": {
                "TO": {
                    "value": "WORLD"
                },
                "THING" : {
                    "value" : "RETRO ENCABULATOR"
                }
            }
        }
    });
    let expected_output = json!({
        "type": "RESPONSE",
        "id": 10,
        "payload": {
            "status": "SE01",
            "headers": {
                "Unsupported-Headers": ["THING"]
            }
        }
    });

    assert_successful(
        &mut handler,
        input.to_string().as_str(),
        Some(expected_output.to_string().as_str()),
    );
}

#[test]
fn handle_malformed_request_type() {
    let (mut handler, _) = gen_frame_handler();

    let input = json!({
        "type": "REQUEST",
        "id": 10,
        "payload": {
            "type": 42,
        }
    });

    let expected_output = json!({
        "type" : "RESPONSE",
        "id" : 10,
        "payload" : {
            "status" : "SE00",
        }
    });

    assert_successful(
        &mut handler,
        input.to_string().as_str(),
        Some(expected_output.to_string().as_str()),
    )
}

#[test]
fn handle_malformed_headers() {
    let (mut handler, _) = gen_frame_handler();

    let input = json!({
        "type": "REQUEST",
        "id": 10,
        "payload": {
            "type": "SAY_HELLO",
            //It shouldn't be an array
            "headers" : [
                "TO", { "value" : "world" }
            ]
        }
    });

    let expected_output = json!({
        "type" : "RESPONSE",
        "id" : 10,
        "payload" : {
            "status" : "SE00",
        }
    });

    assert_successful(
        &mut handler,
        input.to_string().as_str(),
        Some(expected_output.to_string().as_str()),
    )
}

#[test]
fn handle_request_with_payload() {
    let (mut handler, _) = gen_frame_handler();

    let input = json!({
        "type": "REQUEST",
        "id": 10,
        "payload": {
            "type": "PLACE-ORDER",
            "headers" : {
                "PRODUCT-TYPE" : "PHONE",
            },
            "body" : {
                "os": "Android",
                "model": "Pixel 2 XL",
                "brand": "LG",
            }
        }
    });

    let expected_output = json!({
        "type" : "RESPONSE",
        "id" : 10,
        "payload" : {
            "status" : "OK00",
            "body" : 420
        }
    });

    assert_successful(
        &mut handler,
        input.to_string().as_str(),
        Some(expected_output.to_string().as_str()),
    );

    let input = json!({
        "type": "REQUEST",
        "id": 11,
        "payload": {
            "type": "PLACE-ORDER",
            "headers" : {
                "PRODUCT-TYPE" : "PHONE",
            },
            "body" : {
                "os": "iOS",
                "model": "iPhone XL",
                "brand": "Apple",
            }
        }
    });

    let expected_output = json!({
        "type" : "RESPONSE",
        "id" : 11,
        "payload" : {
            "status" : "OK00",
            "body" : 840
        }
    });
    assert_successful(
        &mut handler,
        input.to_string().as_str(),
        Some(expected_output.to_string().as_str()),
    );
}
