use transport_protocol::{config::Config, json::*, *};

#[derive(Default)]
pub struct CounterState {
    invocations: u32,
}

pub fn config() -> Config<Request, Response> {
    let mut state = CounterState::default();

    Config::default().on_request("COUNT", &[], move |_request: Request| {
        state.invocations += 1;

        Response::new(Status::OK(0)).with_body(state.invocations)
    })
}
