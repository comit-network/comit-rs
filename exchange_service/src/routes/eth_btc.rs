use bitcoin_rpc::Address;
use rocket::State;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use treasury_api_client::{create_client, ApiClient};
use types::{Offer, OfferRequestBody, Offers, Symbol, TreasuryApiUrl};
use uuid::Uuid;

#[post("/trades/ETH-BTC/buy-offers", format = "application/json", data = "<offer_request_body>")]
fn post(
    offers: State<Offers>,
    offer_request_body: Json<OfferRequestBody>,
    treasury_api_url: State<TreasuryApiUrl>,
) -> Result<Json<Offer>, BadRequest<String>> {
    let offer_request_body = offer_request_body.into_inner();

    let client = create_client(treasury_api_url.inner());
    let res = client.request_rate(Symbol("ETH-BTC".to_string()));

    match res {
        Ok(rate) => {
            let uid = Uuid::new_v4();

            let offer = Offer {
                symbol: rate.symbol,
                amount: offer_request_body.amount,
                rate: rate.rate,
                uid,
                // TODO: retrieve and use real address
                // This should never be used. Private key is: cSVXkgbkkkjzXV2JMg1zWui4A4dCj55sp9hFoVSUQY9DVh9WWjuj
                address: Address::from("mtgyGsXBNG7Yta5rcMgWH4x9oGE5rm3ty9"),
            };

            let mut result = offers.all_offers.lock().unwrap();
            result.insert(uid, offer);

            let offer = result.get(&uid).unwrap();
            //TODO: avoid the clone
            Ok(Json(offer.clone()))
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
    use rocket::http::{ContentType, Status};
    use rocket_factory::create_rocket_instance;
    use serde_json;
    use types::{OfferRequest, Rate};

    #[test]
    fn given_a_buy_offer_query_should_call_treasury_and_respond() {
        let url = TreasuryApiUrl("stub".to_string());
        let offers = Offers::new();

        let rocket = create_rocket_instance(url, offers);
        let client = rocket::local::Client::new(rocket).unwrap();

        let offer_request = OfferRequest {
            symbol: Symbol("ETH-BTC".to_string()),
            amount: 42,
        };

        let request = client
            .post("/trades/ETH-BTC/buy-offers")
            .header(ContentType::JSON)
            .body(serde_json::to_string(&offer_request).unwrap());
        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);

        let rate = serde_json::from_str::<Rate>(&response.body_string().unwrap()).unwrap();

        assert_eq!(rate.symbol, Symbol("ETH-BTC".to_string()));
    }
}
