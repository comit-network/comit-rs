extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate futures;
extern crate memsocket;
extern crate pretty_env_logger;
extern crate spectral;
extern crate tokio;
extern crate transport_protocol;

#[macro_use]
extern crate log;
extern crate tokio_codec;

use futures::*;
use transport_protocol::*;

mod common;
use common::*;
use transport_protocol::json;

#[test]
fn do_something_on_response() {
    let (mut handler, response_source) = gen_frame_handler();

    let mut future = {
        let mut response_source = response_source.lock().unwrap();

        response_source.on_response_frame(10)
    };

    assert_successful(
        &mut handler,
        r#"{"type":"RESPONSE","id":10,"payload":{"status":"OK00"}}"#,
        None,
    );

    assert_eq!(
        future.poll(),
        Ok(Async::Ready(
            json::Response::new(Status::OK(0)).into_frame(10)
        ))
    );
}
