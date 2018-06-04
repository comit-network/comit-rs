use bitcoin;
use bitcoin::network::constants::Network;
use bitcoin_htlc::Htlc;
use bitcoin_rpc;
use bitcoin_rpc::BlockHeight;
use event_store;
use event_store::EventStore;
use event_store::OfferCreated;
use event_store::OrderCreated;
use event_store::OrderTaken;
use exchange_api_client;
use exchange_api_client::ApiClient;
use exchange_api_client::ExchangeApiUrl;
use exchange_api_client::OfferResponseBody;
use exchange_api_client::OrderRequestBody;
use exchange_api_client::create_client;
use rand::OsRng;
use rocket::State;
use rocket::http::RawStr;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use secret::Secret;
use std::str::FromStr;
use std::sync::Mutex;
use symbol::Symbol;
use uuid::Uuid;
use web3::types::Address as EthAddress;

#[derive(Deserialize)]
pub struct BuyOfferRequestBody {
    amount: u32,
}

#[post("/trades/ETH-BTC/buy-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_buy_offers(
    offer_request_body: Json<BuyOfferRequestBody>,
    url: State<ExchangeApiUrl>,
    _network: State<Network>,
    event_store: State<EventStore>,
) -> Result<Json<OfferResponseBody>, BadRequest<String>> {
    let offer_request_body = offer_request_body.into_inner();
    let symbol = Symbol("ETH-BTC".to_string());

    let client = create_client(url.inner());

    let res = client.create_offer(symbol, offer_request_body.amount);

    match res {
        Ok(offer) => {
            event_store.store_offer_created(OfferCreated::from(offer.clone()))?;
            Ok(Json(offer))
        }
        Err(e) => {
            error!("{:?}", e);

            Err(BadRequest(None))
        }
    }
}

#[derive(Deserialize)]
pub struct BuyOrderRequestBody {
    client_success_address: EthAddress,
    client_refund_address: bitcoin_rpc::Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestToFund {
    uid: Uuid,
    address_to_fund: bitcoin_rpc::Address,
    //TODO: specify amount of BTC
}

const BTC_BLOCKS_IN_24H: u32 = 24 * 60 / 10;

impl From<event_store::Error> for BadRequest<String> {
    fn from(_: event_store::Error) -> Self {
        BadRequest(None)
    }
}

#[post("/trades/ETH-BTC/<trade_id>/buy-orders", format = "application/json",
       data = "<buy_order_request_body>")]
pub fn post_buy_orders(
    trade_id: &RawStr,
    buy_order_request_body: Json<BuyOrderRequestBody>,
    url: State<ExchangeApiUrl>,
    network: State<Network>,
    event_store: State<EventStore>,
    rng: State<Mutex<OsRng>>,
) -> Result<Json<RequestToFund>, BadRequest<String>> {
    let trade_id = match Uuid::parse_str(trade_id.as_ref()) {
        Ok(trade_id) => trade_id,
        Err(_) => return Err(BadRequest(Some("Invalid trade id".to_string()))),
    };

    let offer = match event_store.get_offer_created(&trade_id) {
        Some(offer) => offer,
        None => return Err(BadRequest(None)),
    };

    let buy_order = buy_order_request_body.into_inner();
    let client_success_address = buy_order.client_success_address;
    let client_refund_address = buy_order.client_refund_address;

    let mut secret = {
        let mut rng = rng.lock().unwrap();
        Secret::generate(&mut *rng)
    };

    let order_created_event = OrderCreated {
        uid: trade_id,
        secret: secret.clone(),
        client_success_address: client_success_address.clone(),
        client_refund_address: client_refund_address.clone(),
        long_relative_timelock: BlockHeight::new(BTC_BLOCKS_IN_24H),
    };

    event_store.store_trade_created(order_created_event.clone())?;

    let exchange_client = create_client(url.inner());

    let res = exchange_client.create_order(
        offer.symbol,
        trade_id,
        &OrderRequestBody {
            contract_secret_lock: secret.hash().clone(),
            client_refund_address: client_refund_address.clone(),
            client_success_address: client_success_address.clone(),
            client_contract_time_lock: BlockHeight::new(BTC_BLOCKS_IN_24H),
        },
    );

    let order_response = match res {
        Ok(order_response) => order_response,
        Err(_) => return Err(BadRequest(None)), //TODO: handle error properly
    };

    let exchange_success_address =
        bitcoin::util::address::Address::from_str(
            order_response.exchange_success_address.to_string().as_str(),
        ).expect("Could not convert exchange success address to bitcoin::util::address::Address");

    let client_refund_address =
        bitcoin::util::address::Address::from_str(client_refund_address.to_string().as_str())
            .expect("Could not convert client refund address to bitcoin::util::address::Address");

    let htlc: Htlc = Htlc::new(
        exchange_success_address,
        client_refund_address,
        secret.hash().clone(),
        BTC_BLOCKS_IN_24H,
        network.inner(),
    ).unwrap();

    let order_taken_event = OrderTaken {
        uid: trade_id,
        exchange_contract_time_lock: order_response.exchange_contract_time_lock,
        exchange_refund_address: order_response.exchange_refund_address,
        exchange_success_address: order_response.exchange_success_address,
        htlc: htlc.clone(),
    };

    event_store.store_trade_accepted(order_taken_event)?;

    let htlc_address = bitcoin_rpc::Address::from(htlc.get_htlc_address().clone());

    Ok(Json(RequestToFund {
        uid: trade_id,
        address_to_fund: htlc_address,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use exchange_api_client::ExchangeApiUrl;
    use rocket;
    use rocket::http::*;
    use rocket_factory::create_rocket_instance;
    use serde_json;

    // Secret: 12345678901234567890123456789012
    // Secret hash: 51a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c

    // Sender address: bcrt1qryj6ya9vqpph8w65992nhk64cs890vfy0khsfg
    // Sender pubkey: 020c04eb8cb87485501e30b656f37439ea7866d7c58b3c38161e5793b68e712356
    // Sender pubkey hash: 1925a274ac004373bb5429553bdb55c40e57b124

    // Recipient address: bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap
    // Recipient pubkey: 0298e113cc06bc862ac205f2c0f27ee8c0de98d0716537bbf74e2ea6f38a84d5dc
    // Recipient pubkey hash: c021f17be99c6adfbcba5d38ee0d292c0399d2f5

    // htlc script: 63a82051a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c8876a914c021f17be99c6adfbcba5d38ee0d292c0399d2f567028403b17576a9141925a274ac004373bb5429553bdb55c40e57b1246888ac
    // sha256 of htlc script: e6877a670b46b9913bdaed47084f2db8983c2a22c473f0aea1fa5c2ebc4fd8d4

    #[test]
    fn happy_path_sell_x_btc_for_eth() {
        let url = ExchangeApiUrl("stub".to_string());

        let rocket = create_rocket_instance(url, Network::BitcoinCoreRegtest);
        let client = rocket::local::Client::new(rocket).unwrap();

        let request = client
            .post("/trades/ETH-BTC/buy-offers")
            .header(ContentType::JSON)
            .body(r#"{ "amount": 43 }"#);

        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);
        let offer_response =
            serde_json::from_str::<OfferResponseBody>(&response.body_string().unwrap()).unwrap();

        assert_eq!(
            offer_response.symbol,
            Symbol("ETH-BTC".to_string()),
            "offer_response has correct symbol"
        );
        let uid = offer_response.uid;

        let request = client
            .post(format!("/trades/ETH-BTC/{}/buy-orders", uid).to_string())
            .header(ContentType::JSON)
            // some random addresses I pulled off the internet
            .body(r#"{ "client_success_address": "0x4a965b089f8cb5c75efaa0fbce27ceaaf7722238", "client_refund_address" : "bcrt1qryj6ya9vqpph8w65992nhk64cs890vfy0khsfg" }"#);

        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);

        let funding_request =
            serde_json::from_str::<RequestToFund>(&response.body_string().unwrap()).unwrap();

        assert_eq!(
            funding_request.uid, uid,
            "UID for the funding request is the same as the offer response"
        );

        assert!(
            funding_request
                .address_to_fund
                .to_string()
                .starts_with("bcrt1")
        );
    }
}
