use pretty_env_logger;
use serde_json;
use std::sync::{Arc, Mutex};
use transport_protocol::{
    config::Config,
    json::{self, *},
    Error, *,
};

mod alice_and_bob;
use common::alice_and_bob::{Alice, Bob};
use tokio::runtime::Runtime;

#[derive(Serialize, Deserialize)]
#[serde(tag = "value", content = "parameters")]
enum ThingHeader {
    #[serde(rename = "PHONE")]
    Phone {
        os: String,
        model: String,
        brand: String,
    },
    #[serde(rename = "RETRO ENCABULATOR")]
    RetroEncabulator,
}

#[derive(Serialize, Deserialize)]
struct PriceHeader {
    value: u32,
}

#[derive(Serialize, Deserialize)]
struct SayHelloToHeader {
    value: String,
}

#[derive(Serialize, Deserialize)]
struct SayHelloToTimesHeader {
    value: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "value")]
enum ProductType {
    #[serde(rename = "PHONE")]
    Phone,
    #[serde(rename = "RETRO ENCABULATOR")]
    RetroEncabulator,
}

#[derive(Deserialize)]
struct PhoneSpec {
    pub os: String,
    pub model: String,
    pub brand: String,
}

pub fn gen_frame_handler() -> (
    json::JsonFrameHandler,
    Arc<Mutex<ResponseFrameSource<json::Frame>>>,
) {
    let config = Config::new()
        .on_request("FOO", &[], |_: json::Request| {
            json::Response::new(Status::OK(0))
        })
        .on_request("PING", &[], |_: json::Request| {
            json::Response::new(Status::OK(0))
        })
        .on_request("SAY_HELLO", &["TO", "TIMES"], |request| {
            let to = request
                .get_header::<SayHelloToHeader>("TO")
                .unwrap()
                .unwrap();
            let times = request
                .get_header::<SayHelloToTimesHeader>("TIMES")
                .map(|h| h.unwrap().value)
                .unwrap_or(1);

            let response = (0..times)
                .into_iter()
                .map(|_| to.value.as_str())
                .collect::<Vec<&str>>();

            json::Response::new(Status::OK(0)).with_header("HELLO", response.join(" "))
        })
        .on_request("BUY", &["THING"], |request| {
            let thing = request.get_header("THING").unwrap().unwrap();

            let price = match thing {
                ThingHeader::Phone { .. } => 420,
                ThingHeader::RetroEncabulator => 9001,
            };

            json::Response::new(Status::OK(0)).with_header("PRICE", PriceHeader { value: price })
        })
        .on_request("PLACE-ORDER", &["PRODUCT-TYPE"], |request| {
            let product_type = request.get_header("PRODUCT-TYPE").unwrap().unwrap();

            match product_type {
                ProductType::Phone => {
                    let phone_spec = request.get_body::<PhoneSpec>().unwrap();
                    let price = 420;

                    let price = if phone_spec.os == "iOS" {
                        price * 2
                    } else {
                        price
                    };

                    json::Response::new(Status::OK(0)).with_body(price)
                }
                ProductType::RetroEncabulator => json::Response::new(Status::SE(00)),
            }
        });

    json::JsonFrameHandler::new(config)
}

pub fn assert_successful(handler: &mut json::JsonFrameHandler, input: &str, output: Option<&str>) {
    let _ = pretty_env_logger::try_init();

    let frame =
        serde_json::from_str::<json::Frame>(input).expect("Invalid JSON passed to assertion");

    let response_frame = handler.handle(frame);

    match output {
        Some(output) => {
            let expected_frame = serde_json::from_str::<json::Frame>(output)
                .expect("Invalid JSON passed to assertion");

            match response_frame {
                Ok(Some(generated_frame)) => {
                    assert_eq!(generated_frame, expected_frame);
                }
                Ok(None) => panic!(
                    "Handler did not generated expected frame: {:?}",
                    expected_frame
                ),
                Err(e) => panic!("Handler failed to generate frame: {:?}", e),
            }
        }
        None => match response_frame {
            Ok(Some(_generated_frame)) => panic!("Expected handler to not generate a frame"),
            Ok(None) => {}
            Err(e) => panic!("Handler failed to generate frame: {:?}", e),
        },
    }
}

pub fn setup(config: Config<Request, Response>) -> (Runtime, Alice, Bob) {
    let _ = pretty_env_logger::try_init();

    alice_and_bob::create(config)
}

pub fn assert_error(handler: &mut json::JsonFrameHandler, input: &str, error: Error) {
    let _ = pretty_env_logger::try_init();

    let frame = serde_json::from_str(input).expect("Invalid JSON passed to assertion");

    let actual_output = handler.handle(frame);

    assert_eq!(actual_output, Err(error));
}
