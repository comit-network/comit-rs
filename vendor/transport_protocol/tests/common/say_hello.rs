use futures::future;
use transport_protocol::{config::Config, json::*, *};

#[derive(Serialize, Deserialize)]
pub struct SayHelloToHeader {
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SayHelloToTimesHeader {
    value: u32,
}

impl Default for SayHelloToTimesHeader {
    fn default() -> Self {
        SayHelloToTimesHeader { value: 1 }
    }
}

pub fn config() -> Config<Request, Response> {
    Config::default().on_request("SAY_HELLO", &["TO"], |request: Request| {
        let to = header!(request.get_header::<SayHelloToHeader>("TO"));
        let times = try_header!(request.get_header::<SayHelloToTimesHeader>("TIMES"));

        let response: Vec<&str> = (0..times.value)
            .into_iter()
            .map(|_| to.value.as_str())
            .collect();

        let response = response.join("");

        Box::new(future::ok(
            Response::new(Status::OK(0)).with_header("HELLO", response),
        ))
    })
}
