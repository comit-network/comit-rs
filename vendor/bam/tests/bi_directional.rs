use bam::{config::Config, connection::Connection, json::*, shutdown_handle, *};
use futures::future::{self, Future};
use spectral::prelude::*;
use std::collections::HashMap;

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

    let (alice_server, mut bob_client) = Connection::new(
        Config::default().on_request("PING", &[], |_: Request| {
            Box::new(future::ok(Response::new(Status::OK(0))))
        }),
        JsonFrameCodec::default(),
        alice,
    )
    .start::<JsonFrameHandler>();
    let (alice_server, _alice_shutdown_handle) = shutdown_handle::new(alice_server);

    let (bob_server, mut alice_client) = Connection::new(
        Config::default().on_request("PING", &[], |_: Request| {
            Box::new(future::ok(Response::new(Status::OK(0))))
        }),
        JsonFrameCodec::default(),
        bob,
    )
    .start::<JsonFrameHandler>();
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
