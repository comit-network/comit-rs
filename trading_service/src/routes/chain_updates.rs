use event_store::EventStore;
use event_store::RedeemReady;
use exchange_api_client::ExchangeApiUrl;
use rocket::State;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use std::str::FromStr;
use uuid::Uuid;
use web3::types::Address;

#[derive(Deserialize)]
pub struct RedeemUpdateRequestBody {
    // This could be a vector
    pub uid: Uuid,
    pub address: String,
}

#[post("/chains/ETH/update-redeem-address", format = "application/json",
       data = "<redeem_update_request_body>")]
pub fn post_update_eth_address(
    redeem_update_request_body: Json<RedeemUpdateRequestBody>,
    _url: State<ExchangeApiUrl>,
    event_store: State<EventStore>,
) -> Result<(), BadRequest<String>> {
    let address: Address = match Address::from_str(redeem_update_request_body.address.as_str()) {
        Ok(address) => address,
        Err(_) => return Err(BadRequest(None)),
    };

    match event_store.store_redeem_ready(RedeemReady {
        uid: redeem_update_request_body.uid,
        address,
    }) {
        Ok(_) => return Ok(()),
        Err(_) => return Err(BadRequest(None)),
    }
}
