use bitcoin_rpc::Address;
use rocket::State;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use treasury_api_client::ApiClient;
use treasury_api_client::create_client;
use types::{Offer, OfferRequest, Offers, TreasuryApiUrl};
use uuid::Uuid;

#[post("/ETH-BTC/buy-offer", format = "application/json", data = "<offer_request>")]
fn post(
    offers: State<Offers>,
    offer_request: Json<OfferRequest>,
    treasury_api_url: State<TreasuryApiUrl>,
) -> Result<Json<Offer>, BadRequest<String>> {
    let offer_request = offer_request.into_inner();
    let client = create_client(treasury_api_url.inner());
    let res = client.request_rate(offer_request.symbol);

    match res {
        Ok(rate) => {
            let uid = Uuid::new_v4();

            let offer = Offer {
                symbol: rate.symbol,
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
