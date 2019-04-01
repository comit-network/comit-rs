use bam::{config::Config, json::*, *};
use futures::future;

pub fn config() -> Config<ValidatedIncomingRequest, Response> {
	Config::default().on_request("PING", &[], |_: ValidatedIncomingRequest| {
		Box::new(future::ok(Response::new(Status::OK(0))))
	})
}
