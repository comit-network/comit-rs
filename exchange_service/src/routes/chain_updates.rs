use bitcoin_fee_service;
use bitcoin_fee_service::BitcoinFeeService;
use bitcoin_htlc;
use bitcoin_htlc::UnlockingError;
use bitcoin_rpc;
use bitcoin_support;
use bitcoin_support::PubkeyHash;
use bitcoin_witness::{PrimedInput, PrimedTransaction};
use common_types::secret::Secret;
use event_store::EventStore;
use event_store::TradeId;
use rocket::State;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use std::fmt::Debug;
use std::sync::Arc;

//TODO: move back to eth_btc.rs

#[derive(Deserialize)]
pub struct RedeemBTCNotificationBody {
    pub secret: Secret,
}

fn log_error<E: Debug>(msg: &'static str) -> impl Fn(E) -> BadRequest<String> {
    move |e: E| {
        error!("{}: {:?}", msg, e);
        BadRequest(None)
    }
}

impl From<bitcoin_fee_service::Error> for BadRequest<String> {
    fn from(e: bitcoin_fee_service::Error) -> Self {
        match e {
            bitcoin_fee_service::Error::Unavailable => {
                error!("Unable to retrieve recommended fee. {:?}", e);
                BadRequest(None)
            }
        }
    }
}

#[post("/trades/ETH-BTC/<trade_id>/buy-order-secret-revealed", format = "application/json",
       data = "<redeem_btc_notification_body>")]
pub fn post_revealed_secret(
    redeem_btc_notification_body: Json<RedeemBTCNotificationBody>,
    event_store: State<EventStore>,
    rpc_client: State<Arc<bitcoin_rpc::BitcoinRpcApi>>,
    fee_service: State<Arc<BitcoinFeeService>>,
    btc_exchange_redeem_address: State<bitcoin_support::Address>,
    trade_id: TradeId,
) -> Result<(), BadRequest<String>> {
    let order_taken_event = event_store.get_order_taken_event(&trade_id)?;
    let offer_created_event = event_store.get_offer_created_event(&trade_id)?;
    // TODO: Maybe if this fails we keep the secret around anyway and steal money early?
    let trade_funded_event = event_store.get_trade_funded_event(&trade_id)?;
    let btc_exchange_redeem_address = btc_exchange_redeem_address.inner();
    let secret: Secret = redeem_btc_notification_body.into_inner().secret;
    let exchange_success_address = order_taken_event.exchange_success_address();
    let exchange_success_pubkey_hash: PubkeyHash = exchange_success_address.into();
    let client_refund_pubkey_hash: PubkeyHash = order_taken_event.client_refund_address().into();
    let htlc_txid = trade_funded_event.transaction_id();
    let vout = trade_funded_event.vout();

    let htlc = bitcoin_htlc::Htlc::new(
        exchange_success_pubkey_hash,
        client_refund_pubkey_hash,
        order_taken_event.contract_secret_lock().clone(),
        order_taken_event.client_contract_time_lock().clone().into(),
    );

    let witness_method = {
        let res = htlc.witness_with_secret(
            order_taken_event
                .exchange_success_private_key()
                .secret_key()
                .clone(),
            secret,
        );
        match res {
            Err(e) => match e {
                UnlockingError::WrongSecret { .. } => {
                    error!("Poked with wrong secret: {:?}", e);
                    return Err(BadRequest(Some(format!("{:?}", e).to_string())));
                }
                UnlockingError::WrongSecretKey { .. } => {
                    error!("exchange_success_public_key_hash was inconsistent with exchange_success_private_key");
                    return Err(BadRequest(None));
                }
            },
            Ok(witness_method) => witness_method,
        }
    };

    let primed_txn = PrimedTransaction {
        inputs: vec![
            PrimedInput::new(
                htlc_txid.clone().into(),
                vout,
                offer_created_event.btc_amount(),
                witness_method,
            ),
        ],
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
    //TODO: Store above in event prior to doing rnpc request
    let rpc_transaction =
        bitcoin_rpc::SerializedRawTransaction::from_bitcoin_transaction(redeem_tx).map_err(
            log_error("Failed to convert the transaction into a serialised raw transaction"),
        )?;
    debug!("RPC Transaction: {:?}", rpc_transaction);
    info!(
        "Attempting to redeem HTLC with txid {} for {}",
        htlc_txid, trade_id
    );
    //TODO: Store successful redeem in event
    let redeem_txid = rpc_client
        .send_raw_transaction(rpc_transaction)
        .map_err(log_error("Failed to send connect to bitcoin RPC"))?
        .into_result()
        .map_err(log_error("Failed to send raw transaction to bitcoin RPC"))?;

    info!(
        "HTLC for {} successfully redeemed with {}",
        trade_id, redeem_txid
    );

    Ok(())
}
