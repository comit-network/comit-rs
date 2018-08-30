use transport_protocol::{config::Config, json::*, *};

pub fn config() -> Config<Request, Response> {
    Config::new().on_request("PING", &[], |_: Request| Response::new(Status::OK(0)))
}
