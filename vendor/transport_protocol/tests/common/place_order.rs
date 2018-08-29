use transport_protocol::{config::Config, json::*, *};

pub fn config() -> Config<Request, Response> {
    Config::new().on_request("PLACE-ORDER", &["PRODUCT-TYPE"], |request: Request| {
        let product_type = header!(request.get_header("PRODUCT-TYPE"));

        match product_type {
            ProductType::Phone => {
                let phone_spec = request.get_body::<PhoneSpec>().unwrap();
                let price = 420;

                let price = if phone_spec.os == "iOS" {
                    price * 2
                } else {
                    price
                };

                Response::new(Status::OK(0)).with_body(price)
            }
            ProductType::RetroEncabulator => Response::new(Status::SE(00)),
        }
    })
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "value", content = "parameters")]
pub enum ThingHeader {
    #[serde(rename = "PHONE")]
    Phone {
        os: String,
        model: String,
        brand: String,
    },
    #[serde(rename = "RETRO_ENCABULATOR")]
    RetroEncabulator,
}

#[derive(Serialize, Deserialize)]
pub struct PriceHeader {
    pub value: u32,
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
