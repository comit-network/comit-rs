use bam::{config::Config, json::*, *};
use futures::future;

#[derive(Serialize, Deserialize)]
pub struct SayHelloToHeader {
    value: String,
}

impl SayHelloToHeader {
    pub fn from_header(header: Header) -> Result<Self, serde_json::Error> {
        Ok(SayHelloToHeader {
            value: header.value()?,
        })
    }
}

#[derive(PartialEq, Debug, Eq)]
pub struct HelloResponseHeader {
    pub value: String,
}

impl HelloResponseHeader {
    pub fn from_header(header: Header) -> Result<Self, serde_json::Error> {
        Ok(HelloResponseHeader {
            value: header.value()?,
        })
    }
}

#[derive(Debug)]
struct SayHelloToTimesHeader {
    value: u64,
}

impl SayHelloToTimesHeader {
    pub fn from_header(header: Header) -> Result<Self, serde_json::Error> {
        Ok(SayHelloToTimesHeader {
            value: header.value()?,
        })
    }
}

impl Default for SayHelloToTimesHeader {
    fn default() -> Self {
        SayHelloToTimesHeader { value: 1 }
    }
}

pub fn config() -> Config<ValidatedIncomingRequest, Response> {
    Config::default().on_request(
        "SAY_HELLO",
        &["TO"],
        |mut request: ValidatedIncomingRequest| {
            let to = header!(request.take_header("TO").map(SayHelloToHeader::from_header));
            let times = try_header!(request
                .take_header("TIMES")
                .map(SayHelloToTimesHeader::from_header));

            let response: Vec<&str> = (0..times.value)
                .into_iter()
                .map(|_| to.value.as_str())
                .collect();

            let response = response.join("");

            Box::new(future::ok(Response::new(Status::OK(0)).with_header(
                "HELLO",
                Header::with_json_value(serde_json::to_value(response).unwrap()),
            )))
        },
    )
}
