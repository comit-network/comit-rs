use bam::{config::Config, json::*, *};
use futures::future;

#[derive(Default)]
pub struct CounterState {
	invocations: u32,
}

pub fn config() -> Config<ValidatedIncomingRequest, Response> {
	let mut state = CounterState::default();

	Config::default().on_request("COUNT", &[], move |_request: ValidatedIncomingRequest| {
		state.invocations += 1;

		Box::new(future::ok(
			Response::new(Status::OK(0))
				.with_body(serde_json::to_value(state.invocations).unwrap()),
		))
	})
}
