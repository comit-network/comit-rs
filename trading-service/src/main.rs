#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate lazy_static;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

use rocket_contrib::Json;
use rocket::response::status::BadRequest;
use std::env::var;
use rocket::State;

#[derive(Serialize, Deserialize)]
struct Symbol(String); // Expected format: BTC:LTC

#[derive(Serialize, Deserialize)]
struct OfferRequest {
    symbol: Symbol,
    sell_amount: u32,
}

#[derive(Serialize, Deserialize)]
struct Offer {
    symbol: Symbol,
    rate: f32,
    uid: String,
}

trait ApiClient {
    fn create_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error>;
}

struct DefaultApiClient {
    client: reqwest::Client,
    url: ExchangeApiUrl,
}

impl ApiClient for DefaultApiClient {
    fn create_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        self.client
            .post(self.url.0.as_str())
            .json(offer_request)
            .send()
            .and_then(|mut res| res.json::<Offer>())
    }
}

#[cfg(test)]
fn create_client(url: &ExchangeApiUrl) -> impl ApiClient {
    unimplemented!()
}

#[cfg(not(test))]
fn create_client(url: &ExchangeApiUrl) -> impl ApiClient {
    DefaultApiClient {
        client: reqwest::Client::new(),
        url: url.clone(),
    }
}

#[derive(Clone)]
struct ExchangeApiUrl(String);

#[post("/offers", format = "application/json", data = "<offer_request>")]
fn offers_request(
    offer_request: Json<OfferRequest>,
    url: State<ExchangeApiUrl>,
) -> Result<Json<Offer>, BadRequest<String>> {
    let offer_request = offer_request.into_inner();

    let client = create_client(url.inner());

    let res = client.create_offer(&offer_request);

    match res {
        Ok(offer) => {
            // TODO store in database

            Ok(Json(offer))
        }
        Err(e) => {
            error!("{:?}", e);

            Err(BadRequest(None))
        }
    }
}

fn main() {
    rocket::ignite()
        .mount("/", routes![self::offers_request])
        .manage(ExchangeApiUrl(var("EXCHANGE_SERVICE_URL").unwrap()))
        .launch();
}
