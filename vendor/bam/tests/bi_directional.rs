use bam::{config::Config, connection, json::*, shutdown_handle, *};
use futures::future::{self, Future};
use spectral::prelude::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

struct Ping;

impl From<Ping> for Request {
    fn from(_p: Ping) -> Self {
        Request::new("PING".into(), HashMap::new(), serde_json::Value::Null)
    }
}

#[test]
fn given_two_servers_both_can_ping_each_other() {
    let _ = pretty_env_logger::try_init();

    let (alice, bob) = memsocket::unbounded();

    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    let response_source = Arc::new(Mutex::new(JsonResponseSource::default()));
    let alice_incoming_frames = JsonFrameHandler::create(
        Config::default().on_request("PING", &[], |_: Request| {
            Box::new(future::ok(Response::new(Status::OK(0))))
        }),
        Arc::clone(&response_source),
    );

    let (mut bob_client, bob_outgoing_frames) =
        bam::client::Client::<Frame, Request, Response>::create(response_source);

    let alice_server = connection::new(
        JsonFrameCodec::default(),
        alice,
        alice_incoming_frames,
        bob_outgoing_frames,
    );
    let (alice_server, _alice_shutdown_handle) = shutdown_handle::new(alice_server);

    let response_source = Arc::new(Mutex::new(JsonResponseSource::default()));
    let bob_incoming_frames = JsonFrameHandler::create(
        Config::default().on_request("PING", &[], |_: Request| {
            Box::new(future::ok(Response::new(Status::OK(0))))
        }),
        Arc::clone(&response_source),
    );

    let (mut alice_client, alice_outgoing_frames) =
        bam::client::Client::<Frame, Request, Response>::create(response_source);

    let bob_server = connection::new(
        JsonFrameCodec::default(),
        bob,
        bob_incoming_frames,
        alice_outgoing_frames,
    );
    let (bob_server, _bob_shutdown_handle) = shutdown_handle::new(bob_server);

    runtime.spawn(alice_server.map_err(|_| ()));
    runtime.spawn(bob_server.map_err(|_| ()));

    let alice_response = alice_client.send_request(Ping {}.into()).wait();
    let bob_response = bob_client.send_request(Ping {}.into()).wait();

    assert_that(&alice_response)
        .is_ok()
        .map(|r| r.status())
        .is_equal_to(&Status::OK(0));
    assert_that(&bob_response)
        .is_ok()
        .map(|r| r.status())
        .is_equal_to(&Status::OK(0));
}
