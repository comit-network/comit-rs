use common::place_order::{PriceHeader, ThingHeader};
use transport_protocol::config::Config;

use transport_protocol::{json::*, *};

pub fn config() -> Config<Request, Response> {
    Config::new().on_request("BUY", &["THING"], |request: Request| {
        let thing = header!(request.get_header("THING"));

        let price = match thing {
            ThingHeader::Phone { .. } => 420,
            ThingHeader::RetroEncabulator => 9001,
        };

        Response::new(Status::OK(0)).with_header("PRICE", PriceHeader { value: price })
    })
}
