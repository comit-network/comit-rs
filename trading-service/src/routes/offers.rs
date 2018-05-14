use rocket_contrib::Json;
use rocket::response::status::BadRequest;
use std::env::var;
use rocket::State;
use types::OfferRequest;
use types::ExchangeApiUrl;
use types::Offer;
use exchange_api_client::*;

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
