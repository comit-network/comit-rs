use bitcoin_htlc;
use bitcoin_rpc;
use bitcoin_wallet;
use common_types::BitcoinQuantity;
use common_types::secret::Secret;
use event_store::EventStore;
use event_store::TradeId;
use rocket::State;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct RedeemBTCNotificationBody {
    pub secret: Secret,
}

#[post("/trades/ETH-BTC/<trade_id>/buy-order-secret-revealed", format = "application/json",
       data = "<redeem_btc_notification_body>")]
pub fn post_revealed_secret(
    redeem_btc_notification_body: Json<RedeemBTCNotificationBody>,
    event_store: State<EventStore>,
    rpc_client: State<Arc<bitcoin_rpc::BitcoinRpcApi>>,
    trade_id: TradeId,
) -> Result<(), BadRequest<String>> {
    let order_taken_event = event_store.get_order_taken_event(&trade_id)?;

    let mut secret: Secret = redeem_btc_notification_body.into_inner().secret;

    if secret.hash() != order_taken_event.contract_secret_lock() {
        return Err(BadRequest(Some(
            "the secret didn't match the hash".to_string(),
        )));
    }

    // TODO: Maybe if this fails we keep the secret around anyway and steal money early?
    let trade_funded_event = event_store.get_trade_funded_event(&trade_id)?;

    let htlc_txid = trade_funded_event.transaction_id();
    let vout = trade_funded_event.vout();
    let offer_created_event = event_store.get_offer_created_event(&trade_id)?;
    let input_amount = offer_created_event.btc_amount();
    let fee = BitcoinQuantity::from_satoshi(1000);
    let output_amount = input_amount - fee;

    let exchange_success_address = order_taken_event
        .exchange_success_address()
        .to_bitcoin_address()
        .unwrap();
    let htlc_script = bitcoin_htlc::Htlc::new(
        exchange_success_address.clone(),
        order_taken_event
            .client_refund_address()
            .to_bitcoin_address()
            .unwrap(),
        order_taken_event.contract_secret_lock().clone(),
        order_taken_event.client_contract_time_lock().clone().into(),
        &bitcoin_htlc::Network::BitcoinCoreRegtest,
    ).unwrap()
        .script()
        .clone();

    let redeem_tx = bitcoin_wallet::generate_p2wsh_htlc_redeem_tx(
        htlc_txid,
        vout,
        input_amount,
        output_amount,
        &htlc_script,
        &secret,
        &order_taken_event.exchange_success_private_key(),
        &exchange_success_address,
    ).unwrap();

    //TODO: Store above in event prior to doing rpc request

    let rpc_transaction =
        bitcoin_rpc::SerializedRawTransaction::from_bitcoin_transaction(redeem_tx).unwrap();

    //TODO: Store successful redeem in event
    let _redeem_txid = rpc_client
        .send_raw_transaction(rpc_transaction)
        .unwrap()
        .into_result()
        .unwrap();

    Ok(())
}

#[cfg(test)]
mod test {

    #[test]
    fn tmp() {}
}
