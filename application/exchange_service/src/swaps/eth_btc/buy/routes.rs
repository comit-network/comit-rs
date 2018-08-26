use bitcoin_fee_service::{self, BitcoinFeeService};
use bitcoin_htlc::{self, UnlockingError};
use bitcoin_rpc_client;
use bitcoin_support::{self, BitcoinQuantity, Network, PubkeyHash, ToP2wpkhAddress};
use bitcoin_witness::{PrimedInput, PrimedTransaction};
use common_types::{
    ledger::{
        bitcoin::{self, Bitcoin},
        ethereum::Ethereum,
    },
    secret::{Secret, SecretHash},
    TradingSymbol,
};
use ethereum_htlc;
use ethereum_service;
use ethereum_support;
use event_store::{self, EventStore, InMemoryEventStore};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use secp256k1_support::KeyPair;
use std::{sync::Arc, time::Duration};
use swaps::{
    common::{Error, TradeId},
    events::{ContractDeployed, OfferCreated as OfferState, OfferCreated, OrderTaken, TradeFunded},
};
use treasury_api_client::ApiClient;

impl From<Error> for BadRequest<String> {
    fn from(e: Error) -> Self {
        error!("{:?}", e);
        BadRequest(None)
    }
}

impl From<event_store::Error> for Error {
    fn from(e: event_store::Error) -> Self {
        Error::EventStore(e)
    }
}

impl From<bitcoin_fee_service::Error> for Error {
    fn from(e: bitcoin_fee_service::Error) -> Self {
        Error::FeeService(e)
    }
}

impl From<bitcoin_rpc_client::RpcError> for Error {
    fn from(e: bitcoin_rpc_client::RpcError) -> Self {
        Error::BitcoinRpc(e)
    }
}

impl From<ethereum_service::Error> for Error {
    fn from(e: ethereum_service::Error) -> Self {
        Error::EthereumService(e)
    }
}

impl From<UnlockingError> for Error {
    fn from(e: UnlockingError) -> Self {
        match e {
            UnlockingError::WrongSecret { .. } => {
                Error::Unlocking(format!("{:?}", e).to_string())
            }
            UnlockingError::WrongKeyPair { .. } => {
                Error::Unlocking("exchange_success_public_key_hash was inconsistent with exchange_success_private_key".to_string())
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfferRequestBody {
    pub amount: f64,
}

#[post("/trades/ETH-BTC/buy-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_buy_offers(
    offer_request_body: Json<OfferRequestBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
    treasury_api_client: State<Arc<ApiClient>>,
) -> Result<Json<OfferState<Ethereum, Bitcoin>>, BadRequest<String>> {
    let offer_state = handle_post_buy_offers(
        offer_request_body.into_inner(),
        event_store.inner(),
        treasury_api_client.inner(),
    )?;

    Ok(Json(offer_state)) // offer_event is the same than state.
}

fn handle_post_buy_offers(
    offer_request_body: OfferRequestBody,
    event_store: &InMemoryEventStore<TradeId>,
    treasury_api_client: &Arc<ApiClient>,
) -> Result<OfferState<Ethereum, Bitcoin>, Error> {
    let buy_amount = ethereum_support::EthereumQuantity::from_eth(offer_request_body.amount);

    let rate_response_body = treasury_api_client
        .request_rate(TradingSymbol::ETH_BTC)
        .map_err(Error::TreasuryService)?;
    let sell_amount =
        BitcoinQuantity::from_bitcoin(rate_response_body.rate * buy_amount.ethereum());

    let offer_event = OfferCreated::new(
        rate_response_body.rate,
        buy_amount,
        sell_amount,
        TradingSymbol::ETH_BTC,
    );

    event_store.add_event(offer_event.uid, offer_event.clone())?;

    info!("Created new offer: {:?}", offer_event);

    Ok(offer_event)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderRequestBody {
    pub contract_secret_lock: SecretHash,
    pub client_contract_time_lock: bitcoin_rpc_client::BlockHeight,

    pub client_refund_address: bitcoin_rpc_client::Address,
    pub client_success_address: ethereum_support::Address,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderTakenResponseBody {
    pub exchange_refund_address: ethereum_support::Address,
    pub exchange_success_address: bitcoin_rpc_client::Address,
    pub exchange_contract_time_lock: u64,
}

impl From<OrderTaken<Ethereum, Bitcoin>> for OrderTakenResponseBody {
    fn from(order_taken_event: OrderTaken<Ethereum, Bitcoin>) -> Self {
        OrderTakenResponseBody {
            exchange_refund_address: order_taken_event.exchange_refund_address.into(),
            exchange_success_address: order_taken_event.exchange_success_address.into(),
            exchange_contract_time_lock: order_taken_event.exchange_contract_time_lock.as_secs(),
        }
    }
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-orders",
    format = "application/json",
    data = "<order_request_body>"
)]
pub fn post_buy_orders(
    trade_id: TradeId,
    order_request_body: Json<OrderRequestBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
    exchange_success_keypair: State<KeyPair>,
    exchange_refund_address: State<ethereum_support::Address>,
    network: State<Network>,
) -> Result<Json<OrderTakenResponseBody>, BadRequest<String>> {
    let order_taken_response_body = handle_post_buy_orders(
        trade_id,
        order_request_body.into_inner(),
        event_store.inner(),
        exchange_success_keypair.inner(),
        exchange_refund_address.inner(),
        network.inner(),
    )?;
    Ok(Json(order_taken_response_body))
}

fn handle_post_buy_orders(
    trade_id: TradeId,
    order_request_body: OrderRequestBody,
    event_store: &InMemoryEventStore<TradeId>,
    exchange_success_keypair: &KeyPair,
    exchange_refund_address: &ethereum_support::Address,
    network: &Network,
) -> Result<OrderTakenResponseBody, Error> {
    // Receive trade information
    // - Hashed Secret
    // - Client refund address (BTC)
    // - timeout (BTC)
    // - Client success address (ETH)
    // = generates exchange refund address
    // -> returns ETH HTLC data (exchange refund address + ETH timeout)
    let client_refund_address: bitcoin_support::Address =
        order_request_body.client_refund_address.into();
    //TODO: clean up, should not need to do address>pub_key>address
    let exchange_success_address = bitcoin_support::Address::from(
        exchange_success_keypair
            .public_key()
            .clone()
            .to_p2wpkh_address(*network),
    );

    let twelve_hours = Duration::new(60 * 60 * 12, 0);

    let order_taken = OrderTaken {
        uid: trade_id,
        contract_secret_lock: order_request_body.contract_secret_lock,
        client_contract_time_lock: order_request_body.client_contract_time_lock,
        exchange_contract_time_lock: twelve_hours,
        client_refund_address,
        client_success_address: order_request_body.client_success_address,
        exchange_refund_address: *exchange_refund_address,
        exchange_success_address,
        exchange_success_keypair: exchange_success_keypair.clone(),
    };

    event_store.add_event(trade_id, order_taken.clone())?;
    Ok(order_taken.into())
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-htlc-funded",
    format = "application/json",
    data = "<htlc_identifier>"
)]
pub fn post_orders_funding(
    trade_id: TradeId,
    htlc_identifier: Json<bitcoin::HtlcId>,
    event_store: State<InMemoryEventStore<TradeId>>,
    ethereum_service: State<Arc<ethereum_service::EthereumService>>,
) -> Result<(), BadRequest<String>> {
    handle_post_orders_funding(
        trade_id,
        htlc_identifier.into_inner(),
        event_store.inner(),
        ethereum_service.inner(),
    )?;
    Ok(())
}

fn handle_post_orders_funding(
    trade_id: TradeId,
    htlc_identifier: bitcoin::HtlcId,
    event_store: &InMemoryEventStore<TradeId>,
    ethereum_service: &Arc<ethereum_service::EthereumService>,
) -> Result<(), Error> {
    let trade_funded: TradeFunded<Bitcoin> = TradeFunded {
        uid: trade_id,
        htlc_identifier,
    };

    event_store.add_event(trade_id.clone(), trade_funded)?;

    let order_taken = event_store.get_event::<OrderTaken<Ethereum, Bitcoin>>(trade_id.clone())?;

    let htlc = ethereum_htlc::Htlc::new(
        order_taken.exchange_contract_time_lock,
        order_taken.exchange_refund_address,
        order_taken.client_success_address,
        order_taken.contract_secret_lock.clone(),
    );

    let offer_created_event =
        event_store.get_event::<OfferCreated<Ethereum, Bitcoin>>(trade_id.clone())?;

    let htlc_funding = offer_created_event.buy_amount.wei();

    let tx_id = ethereum_service.deploy_htlc(htlc, htlc_funding)?;
    let deployed: ContractDeployed<Ethereum> = ContractDeployed::new(trade_id, tx_id.to_string());

    event_store.add_event(trade_id, deployed)?;

    Ok(())
}

#[derive(Deserialize)]
pub struct RedeemBTCNotificationBody {
    pub secret: Secret,
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-secret-revealed",
    format = "application/json",
    data = "<redeem_btc_notification_body>"
)]
pub fn post_revealed_secret(
    redeem_btc_notification_body: Json<RedeemBTCNotificationBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
    rpc_client: State<Arc<bitcoin_rpc_client::BitcoinRpcApi>>,
    fee_service: State<Arc<BitcoinFeeService>>,
    btc_exchange_redeem_address: State<bitcoin_support::Address>,
    trade_id: TradeId,
) -> Result<(), BadRequest<String>> {
    handle_post_revealed_secret(
        redeem_btc_notification_body.into_inner(),
        event_store.inner(),
        rpc_client.inner(),
        fee_service.inner(),
        btc_exchange_redeem_address.inner(),
        trade_id,
    )?;

    Ok(())
}

fn handle_post_revealed_secret(
    redeem_btc_notification_body: RedeemBTCNotificationBody,
    event_store: &InMemoryEventStore<TradeId>,
    rpc_client: &Arc<bitcoin_rpc_client::BitcoinRpcApi>,
    fee_service: &Arc<BitcoinFeeService>,
    btc_exchange_redeem_address: &bitcoin_support::Address,
    trade_id: TradeId,
) -> Result<(), Error> {
    let order_taken_event =
        event_store.get_event::<OrderTaken<Ethereum, Bitcoin>>(trade_id.clone())?;
    let offer_created_event =
        event_store.get_event::<OfferCreated<Ethereum, Bitcoin>>(trade_id.clone())?;
    // TODO: Maybe if this fails we keep the secret around anyway and steal money early?
    let trade_funded_event = event_store.get_event::<TradeFunded<Bitcoin>>(trade_id.clone())?;
    let secret: Secret = redeem_btc_notification_body.secret;
    let exchange_success_address = order_taken_event.exchange_success_address;
    let exchange_success_pubkey_hash: PubkeyHash = exchange_success_address.into();
    let exchange_success_keypair = order_taken_event.exchange_success_keypair;

    let client_refund_pubkey_hash: PubkeyHash = order_taken_event.client_refund_address.into();
    let htlc_txid = trade_funded_event.htlc_identifier.transaction_id;
    let vout = trade_funded_event.htlc_identifier.vout;

    let htlc = bitcoin_htlc::Htlc::new(
        exchange_success_pubkey_hash,
        client_refund_pubkey_hash,
        order_taken_event.contract_secret_lock.clone(),
        order_taken_event.client_contract_time_lock.clone().into(),
    );

    htlc.can_be_unlocked_with(&secret, &exchange_success_keypair)?;

    let unlocking_parameters = htlc.unlock_with_secret(exchange_success_keypair.clone(), secret);

    let primed_txn = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            htlc_txid.clone().into(),
            vout,
            offer_created_event.sell_amount,
            unlocking_parameters,
        )],
        output_address: btc_exchange_redeem_address.clone(),
        locktime: 0,
    };

    let total_input_value = primed_txn.total_input_value();

    let rate = fee_service.get_recommended_fee()?;
    let redeem_tx = primed_txn.sign_with_rate(rate);

    debug!(
        "Redeem {} (input: {}, vout: {}) to {} (output: {})",
        htlc_txid,
        total_input_value,
        vout,
        redeem_tx.txid(),
        redeem_tx.output[0].value
    );
    //TODO: Store above in event prior to doing rpc request
    let rpc_transaction = bitcoin_rpc_client::SerializedRawTransaction::from(redeem_tx);
    debug!("RPC Transaction: {:?}", rpc_transaction);
    info!(
        "Attempting to redeem HTLC with txid {} for {}",
        htlc_txid, trade_id
    );
    //TODO: Store successful redeem in event
    let redeem_txid = rpc_client
        .send_raw_transaction(rpc_transaction)
        .map_err(Error::BitcoinNode)??;

    info!(
        "HTLC for {} successfully redeemed with {}",
        trade_id, redeem_txid
    );

    Ok(())
}
