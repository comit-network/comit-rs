use bitcoin_htlc::{self, Htlc as BtcHtlc};
use bitcoin_rpc::{self, BlockHeight};
use bitcoin_support::{self, BitcoinQuantity, Network, PubkeyHash};
use ethereum_support::{self, EthereumQuantity};
use event_store::{self, EventStore, InMemoryEventStore};
use events::{ContractDeployed, OfferCreated, OrderCreated, OrderTaken, TradeId};
use exchange_api_client::{ApiClient, OfferResponseBody, OrderRequestBody};
use logging;
use rand::OsRng;
use reqwest;
use rocket::{http::RawStr, request::FromParam, response::status::BadRequest, State};
use rocket_contrib::Json;
use secret::Secret;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};
use symbol::Symbol;
use uuid::{self, Uuid};

impl<'a> FromParam<'a> for TradeId {
    type Error = uuid::ParseError;

    fn from_param(param: &RawStr) -> Result<Self, <Self as FromParam>::Error> {
        Uuid::parse_str(param.as_str()).map(|uid| {
            logging::set_context(&uid);
            TradeId::from(uid)
        })
    }
}

#[derive(Debug)]
pub enum Error {
    EventStore(event_store::Error),
    ExchangeService(reqwest::Error),
}

#[derive(Deserialize)]
pub struct BuyOfferRequestBody {
    amount: f64,
}

#[post("/trades/ETH-BTC/buy-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_buy_offers(
    offer_request_body: Json<BuyOfferRequestBody>,
    client: State<Arc<ApiClient>>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<OfferResponseBody>, BadRequest<String>> {
    let offer_request_body = offer_request_body.into_inner();
    let symbol = Symbol("ETH-BTC".to_string());

    let res = client.create_offer(symbol, offer_request_body.amount);

    match res {
        Ok(offer) => {
            let id = offer.uid.clone();
            let event = OfferCreated::from(offer.clone());

            event_store.add_event(id, event).map_err(Error::EventStore)?;
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
    client_success_address: ethereum_support::Address,
    // TODO: this forces the trading-cli to have a dependency on bitcoin_rpc.
    // I think we should avoid it and push for a dependency on rust-bitcoin instead
    // However, rust-bitcoin addresses do not seem to deserialize:
    // the trait `serde::Deserialize<'_>` is not implemented for `bitcoin::util::address::Address`
    client_refund_address: bitcoin_rpc::Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestToFund {
    address_to_fund: bitcoin_rpc::Address,
    btc_amount: BitcoinQuantity,
    eth_amount: EthereumQuantity,
}

const BTC_BLOCKS_IN_24H: u32 = 24 * 60 / 10;

impl From<Error> for BadRequest<String> {
    fn from(e: Error) -> Self {
        error!("Error: {:?}", e);
        BadRequest(None)
    }
}

impl From<event_store::Error> for Error {
    fn from(e: event_store::Error) -> Self {
        error!("Error: {:?}", e);
        Error::EventStore(e)
    }
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-orders",
    format = "application/json",
    data = "<buy_order_request_body>"
)]
pub fn post_buy_orders(
    trade_id: TradeId,
    buy_order_request_body: Json<BuyOrderRequestBody>,
    client: State<Arc<ApiClient>>,
    network: State<Network>,
    event_store: State<InMemoryEventStore<TradeId>>,
    rng: State<Mutex<OsRng>>,
) -> Result<Json<RequestToFund>, BadRequest<String>> {
    let request_to_fund = handle_buy_orders(
        client.inner(),
        event_store.inner(),
        rng.inner(),
        network.inner(),
        trade_id,
        buy_order_request_body.into_inner(),
    )?;

    Ok(Json(request_to_fund))
}

fn handle_buy_orders(
    client: &Arc<ApiClient>,
    event_store: &InMemoryEventStore<TradeId>,
    rng: &Mutex<OsRng>,
    network: &Network,
    trade_id: TradeId,
    buy_order: BuyOrderRequestBody,
) -> Result<RequestToFund, Error> {
    let offer = event_store.get_event::<OfferCreated>(trade_id)?;

    let client_success_address = buy_order.client_success_address;
    let client_refund_address = buy_order.client_refund_address;

    let secret = {
        let mut rng = rng.lock().unwrap();
        Secret::generate(&mut *rng)
    };

    //TODO: Remove before prod
    debug!("Secret: {:x}", secret);

    let order_created_event = OrderCreated {
        uid: trade_id,
        secret: secret.clone(),
        client_success_address: client_success_address.clone(),
        client_refund_address: client_refund_address.clone(),
        long_relative_timelock: BlockHeight::new(BTC_BLOCKS_IN_24H),
    };

    event_store.add_event(trade_id, order_created_event.clone())?;

    let order_response = client
        .create_order(
            offer.symbol,
            trade_id,
            &OrderRequestBody {
                contract_secret_lock: secret.hash(),
                client_refund_address: client_refund_address.clone(),
                client_success_address: client_success_address.clone(),
                client_contract_time_lock: BlockHeight::new(BTC_BLOCKS_IN_24H),
            },
        )
        .map_err(Error::ExchangeService)?;

    let exchange_success_address =
        bitcoin_support::Address::from_str(
            order_response.exchange_success_address.to_string().as_str(),
        ).expect("Could not convert exchange success address to bitcoin::util::address::Address");

    let client_refund_address =
        bitcoin_support::Address::from_str(client_refund_address.to_string().as_str())
            .expect("Could not convert client refund address to bitcoin::util::address::Address");

    let exchange_success_pubkey_hash = PubkeyHash::from(exchange_success_address);
    let client_refund_pubkey_hash = PubkeyHash::from(client_refund_address);

    let htlc: BtcHtlc = BtcHtlc::new(
        exchange_success_pubkey_hash,
        client_refund_pubkey_hash,
        secret.hash(),
        BTC_BLOCKS_IN_24H,
    );

    let order_taken_event = OrderTaken {
        uid: trade_id,
        exchange_contract_time_lock: order_response.exchange_contract_time_lock,
        exchange_refund_address: order_response.exchange_refund_address,
        exchange_success_address: order_response.exchange_success_address,
        htlc: htlc.clone(),
    };

    event_store.add_event(trade_id, order_taken_event)?;

    let offer = event_store.get_event::<OfferCreated>(trade_id).unwrap();

    let htlc_address = bitcoin_rpc::Address::from(htlc.compute_address(network.clone()));

    Ok(RequestToFund {
        address_to_fund: htlc_address,
        eth_amount: offer.eth_amount,
        btc_amount: offer.btc_amount,
    })
}

#[derive(Deserialize)]
pub struct ContractDeployedRequestBody {
    pub contract_address: ethereum_support::Address,
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-contract-deployed",
    format = "application/json",
    data = "<contract_deployed_request_body>"
)]
pub fn post_contract_deployed(
    trade_id: TradeId,
    contract_deployed_request_body: Json<ContractDeployedRequestBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<(), BadRequest<String>> {
    handle_post_contract_deployed(
        event_store.inner(),
        trade_id,
        contract_deployed_request_body.into_inner().contract_address,
    )?;

    Ok(())
}

fn handle_post_contract_deployed(
    event_store: &InMemoryEventStore<TradeId>,
    uid: TradeId,
    address: ethereum_support::Address,
) -> Result<(), Error> {
    event_store.add_event(uid, ContractDeployed { uid, address })?;

    Ok(())
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RedeemDetails {
    address: ethereum_support::Address,
    data: bitcoin_htlc::secret::Secret,
    gas: u64,
}

#[get("/trades/ETH-BTC/<trade_id>/redeem-orders")]
pub fn get_redeem_orders(
    trade_id: TradeId,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<RedeemDetails>, BadRequest<String>> {
    let details = handle_get_redeem_orders(event_store.inner(), trade_id)?;

    Ok(Json(details))
}

fn handle_get_redeem_orders(
    event_store: &InMemoryEventStore<TradeId>,
    trade_id: TradeId,
) -> Result<RedeemDetails, Error> {
    let address = event_store.get_event::<ContractDeployed>(trade_id)?.address;
    let secret = event_store.get_event::<OrderCreated>(trade_id)?.secret;

    Ok(RedeemDetails {
        address,
        data: secret,
        // TODO: check how much gas we should tell the customer to pay
        gas: 35000,
    })
}

#[cfg(test)]
mod tests {
    use event_store::InMemoryEventStore;
    use rocket::{self, http::*};
    use rocket_factory::create_rocket_instance;
    use serde_json;

    use super::*;
    use exchange_api_client::FakeApiClient;

    // Secret: 12345678901234567890123456789012
    // Secret hash: 51a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c

    // Sender address: bcrt1qryj6ya9vqpph8w65992nhk64cs890vfy0khsfg
    // Sender pubkey: 020c04eb8cb87485501e30b656f37439ea7866d7c58b3c38161e5793b68e712356
    // Sender pubkey hash: 1925a274ac004373bb5429553bdb55c40e57b124

    // Recipient address: bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap
    // Recipient pubkey: 0298e113cc06bc862ac205f2c0f27ee8c0de98d0716537bbf74e2ea6f38a84d5dc
    // Recipient pubkey hash: c021f17be99c6adfbcba5d38ee0d292c0399d2f5

    // htlc script: 63a82051a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c8876a914c021f17be99c6adfbcba5d38ee0d292c0399d2f567028403b17576a9141925a274ac004373bb5429553bdb55c40e57b1246888ac
    #[test]
    fn happy_path_sell_x_btc_for_eth() {
        let rocket = create_rocket_instance(
            Network::Testnet,
            InMemoryEventStore::new(),
            Arc::new(FakeApiClient::new()),
        );
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
            .post(format!("/trades/ETH-BTC/{}/buy-orders", uid))
            .header(ContentType::JSON)
            // some random addresses I pulled off the internet
            .body(r#"{ "client_success_address": "0x4a965b089f8cb5c75efaa0fbce27ceaaf7722238", "client_refund_address" : "tb1qj3z3ymhfawvdp4rphamc7777xargzufztd44fv" }"#);

        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);

        let funding_request =
            serde_json::from_str::<RequestToFund>(&response.body_string().unwrap()).unwrap();

        assert!(
            funding_request
                .address_to_fund
                .to_string()
                .starts_with("tb1")
        );

        let request = client
            .post(format!(
                "/trades/ETH-BTC/{}/buy-order-contract-deployed",
                uid
            ))
            .header(ContentType::JSON)
            .body(r#"{ "contract_address" : "0x00a329c0648769a73afac7f9381e08fb43dbea72" }"#);

        let response = request.dispatch();

        assert_eq!(
            response.status(),
            Status::Ok,
            "buy-order-contract-deployed call is successful"
        );

        let request = client.get(format!("/trades/ETH-BTC/{}/redeem-orders", uid).to_string());

        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);

        let _redeem_details =
            serde_json::from_str::<RedeemDetails>(&response.body_string().unwrap()).unwrap();
    }

    // sha256 of htlc script: e6877a670b46b9913bdaed47084f2db8983c2a22c473f0aea1fa5c2ebc4fd8d4
}
