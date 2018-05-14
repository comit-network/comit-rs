use exchange_api_client::*;
use rocket::State;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use std::env::var;
use types::ExchangeApiUrl;
use types::Offer;
use types::OfferRequest;

#[post("/offers", format = "application/json", data = "<offer_request>")]
pub fn post(
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

#[cfg(test)]
mod tests {

    use rocket;
    use rocket::http::*;
    use rocket_factory::create_rocket_instance;
    use types::ExchangeApiUrl;

    #[test]
    fn given_an_offer_from_exchange_should_attach_hash_of_secret() {
        let url = ExchangeApiUrl("stub".to_string());

        let rocket = create_rocket_instance(url);
        let client = rocket::local::Client::new(rocket).unwrap();

        let request = client.post("/offers").header(ContentType::JSON).body(
            r#"{
            "symbol": "ETH:BTC",
            "sell_amount": 0
        }"#,
        );

        let response = request.dispatch();

        assert_eq!(response.status(), Status::Ok)
    }
}
