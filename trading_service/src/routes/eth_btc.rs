use exchange_api_client::ApiClient;
use exchange_api_client::ExchangeApiUrl;
use exchange_api_client::*;
use offer::{Offer, OfferRepository};
use rocket::State;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use symbol::Symbol;

#[derive(Deserialize)]
pub struct BuyOfferRequestBody {
    amount: u32,
}

#[post("/trades/ETH-BTC/buy-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_buy_offers(
    offer_request_body: Json<BuyOfferRequestBody>,
    url: State<ExchangeApiUrl>,
    offer_repository: State<OfferRepository>,
) -> Result<Json<Offer>, BadRequest<String>> {
    let offer_request_body = offer_request_body.into_inner();
    let symbol = Symbol("ETH-BTC".to_string());

    let client = create_client(url.inner());

    let res = client.create_offer(symbol, offer_request_body.amount);

    match res {
        Ok(offer) => {
            offer_repository.insert(&offer);

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
    use super::*;
    use exchange_api_client::ExchangeApiUrl;
    use offer::Offer;
    use offer::OfferRepository;
    use rocket;
    use rocket::http::*;
    use rocket_factory::create_rocket_instance;
    use serde_json;

    #[test]
    fn given_an_offer_from_exchange_should_respond_with_offer() {
        let url = ExchangeApiUrl("stub".to_string());
        let offer_repository = OfferRepository::new();

        let rocket = create_rocket_instance(url, offer_repository);
        let client = rocket::local::Client::new(rocket).unwrap();

        let request = client
            .post("/trades/ETH-BTC/buy-offers")
            .header(ContentType::JSON)
            .body(r#"{ "amount": 43 }"#);

        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);
        let offer_response =
            serde_json::from_str::<Offer>(&response.body_string().unwrap()).unwrap();

        assert_eq!(offer_response.symbol, Symbol("ETH-BTC".to_string()));
    }
}
