use crate::common::place_order::{PriceHeader, ThingHeader};
use bam::{config::Config, json::*, *};
use futures::future;

pub fn config() -> Config<ValidatedIncomingRequest, Response> {
    Config::default().on_request(
        "BUY",
        &["THING"],
        |mut request: ValidatedIncomingRequest| {
            let price = match header!(request.take_header("THING").map(ThingHeader::from_header)) {
                ThingHeader::Phone { .. } => 420,
                ThingHeader::RetroEncabulator => 9001,
                ThingHeader::Unknown { .. } => panic!("unexpected unknown thingy"),
            };

            Box::new(future::ok(Response::new(Status::OK(0)).with_header(
                "PRICE",
                PriceHeader { value: price }.to_header().unwrap(),
            )))
        },
    )
}
