use exchange_api_client::*;
use rand::OsRng;
use rocket::State;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use secret::Secret;
use std::sync::Mutex;
use types::*;

#[post("/offers", format = "application/json", data = "<offer_request>")]
pub fn post(
    offer_request: Json<OfferRequest>,
    url: State<ExchangeApiUrl>,
    offers: State<Offers>,
    rng: State<Mutex<OsRng>>,
) -> Result<Json<SwapProposal>, BadRequest<String>> {
    let offer_request = offer_request.into_inner();

    let client = create_client(url.inner());

    let res = client.create_offer(&offer_request);

    match res {
        Ok(exchange_offer) => {
            let mut rng = rng.lock().unwrap();

            let mut secret = Secret::generate(&mut *rng);

            let swap_proposal = {
                let secret_hash = secret.hash();
                SwapProposal::from_exchange_offer(exchange_offer, secret_hash.clone())
            };

            let swap_data = SwapData::new(swap_proposal.clone(), secret);

            {
                //TODO: make it nicer by creating method Offer::insert()
                let mut offers = offers.all_offers.lock().unwrap();
                offers.insert(swap_data.uid(), swap_data);
            }

            Ok(Json(swap_proposal))
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
    use rocket;
    use rocket::http::*;
    use rocket_factory::create_rocket_instance;
    use serde_json;
    use types::Symbol;

    #[test]
    fn given_an_offer_from_exchange_should_respond_with_offer() {
        let url = ExchangeApiUrl("stub".to_string());
        let offers = Offers::new();

        let rocket = create_rocket_instance(url, offers);
        let client = rocket::local::Client::new(rocket).unwrap();

        let request = client.post("/offers").header(ContentType::JSON).body(
            r#"{
            "symbol": "ETH:BTC",
            "sell_amount": 0
        }"#,
        );

        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);
        let offer_response: SwapProposal =
            serde_json::from_str::<SwapProposal>(&response.body_string().unwrap()).unwrap();

        assert_eq!(offer_response.symbol, Symbol("ETH:BTC".to_string()));

        // 32 bytes -> 64 hex characters
        assert_eq!(offer_response.secret_hash.as_hex().len(), 64);
    }
}
