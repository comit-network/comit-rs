use bam::{config::Config, json::*, *};
use futures::future;

#[derive(Default)]
pub struct CounterState {
    invocations: u32,
}

pub fn config() -> Config<Request, Response> {
    let mut state = CounterState::default();

    Config::default().on_request("COUNT", &[], move |_request: Request| {
        state.invocations += 1;

        Box::new(future::ok(
            Response::new(Status::OK(0)).with_body(state.invocations),
        ))
    })
}
