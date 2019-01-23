use bam::{config::Config, json::*, *};
use futures::future;

pub fn config() -> Config<ValidatedIncomingRequest, Response> {
    Config::default().on_request(
        "PLACE-ORDER",
        &["PRODUCT-TYPE"],
        |mut request: ValidatedIncomingRequest| {
            let product_type = header!(request
                .take_header("PRODUCT-TYPE")
                .map(ProductType::from_header));

            let response = match product_type {
                ProductType::Phone => {
                    let phone_spec = body!(request.take_body_as::<PhoneSpec>());
                    let price = 420;

                    let price = if phone_spec.os == "iOS" {
                        price * 2
                    } else {
                        price
                    };

                    Response::new(Status::OK(0)).with_body(serde_json::to_value(price).unwrap())
                }
                _ => Response::new(Status::SE(00)),
            };

            Box::new(future::ok(response))
        },
    )
}

pub enum ThingHeader {
    Phone {
        os: String,
        model: String,
        brand: String,
    },
    RetroEncabulator,
    Unknown {
        name: String,
    },
}

pub enum ThingHeaderError {
    MissingParameter(String),
    Serde(serde_json::Error),
}

impl ThingHeader {
    pub fn from_header(mut header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "PHONE" => ThingHeader::Phone {
                os: header.take_parameter("os")?,
                model: header.take_parameter("model")?,
                brand: header.take_parameter("brand")?,
            },
            "RETRO_ENCABULATOR" => ThingHeader::RetroEncabulator,
            other => ThingHeader::Unknown {
                name: other.to_string(),
            },
        })
    }
}

pub struct PriceHeader {
    pub value: u32,
}

impl PriceHeader {
    pub fn to_header(&self) -> Result<Header, serde_json::Error> {
        Ok(Header::with_value(self.value)?)
    }
}

enum ProductType {
    Phone,
    RetroEncabulator,
    Unknown,
}

impl ProductType {
    pub fn from_header(header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "PHONE" => ProductType::Phone,
            "RETRO_ENCABULATOR" => ProductType::RetroEncabulator,
            _ => ProductType::Unknown,
        })
    }
}

#[derive(Deserialize)]
struct PhoneSpec {
    pub os: String,
    pub model: String,
    pub brand: String,
}
