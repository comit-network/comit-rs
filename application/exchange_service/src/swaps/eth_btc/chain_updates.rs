use super::{events::*, routes::Error};
use bitcoin_fee_service::BitcoinFeeService;
use bitcoin_htlc::{self, UnlockingError};
use bitcoin_rpc;
use bitcoin_support::{self, PubkeyHash};
use bitcoin_witness::{PrimedInput, PrimedTransaction};
use common_types::secret::Secret;
use event_store::{EventStore, InMemoryEventStore};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::{fmt::Debug, sync::Arc};
use swaps::TradeId;

//TODO: move back to eth_btc.rs

#[derive(Deserialize)]
pub struct RedeemBTCNotificationBody {
    pub secret: Secret,
}

fn log_error<E: Debug>(msg: &'static str) -> impl Fn(E) -> Error {
    move |e: E| Error::AdHoc(format!("{}: {:?}", msg, e))
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-secret-revealed",
    format = "application/json",
    data = "<redeem_btc_notification_body>"
)]
pub fn post_revealed_secret(
    redeem_btc_notification_body: Json<RedeemBTCNotificationBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
    rpc_client: State<Arc<bitcoin_rpc::BitcoinRpcApi>>,
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
    rpc_client: &Arc<bitcoin_rpc::BitcoinRpcApi>,
    fee_service: &Arc<BitcoinFeeService>,
    btc_exchange_redeem_address: &bitcoin_support::Address,
    trade_id: TradeId,
) -> Result<(), Error> {
    let order_taken_event = event_store.get_event::<OrderTaken>(trade_id.clone())?;
    let offer_created_event = event_store.get_event::<OfferCreated>(trade_id.clone())?;
    // TODO: Maybe if this fails we keep the secret around anyway and steal money early?
    let trade_funded_event = event_store.get_event::<TradeFunded>(trade_id.clone())?;
    let secret: Secret = redeem_btc_notification_body.secret;
    let exchange_success_address = order_taken_event.exchange_success_address;
    let exchange_success_pubkey_hash: PubkeyHash = exchange_success_address.into();
    let exchange_success_keypair = order_taken_event.exchange_success_keypair;

    let client_refund_pubkey_hash: PubkeyHash = order_taken_event.client_refund_address.into();
    let htlc_txid = trade_funded_event.transaction_id;
    let vout = trade_funded_event.vout;

    let htlc = bitcoin_htlc::Htlc::new(
        exchange_success_pubkey_hash,
        client_refund_pubkey_hash,
        order_taken_event.contract_secret_lock.clone(),
        order_taken_event.client_contract_time_lock.clone().into(),
    );

    htlc.can_be_unlocked_with(&secret, &exchange_success_keypair)
        .map_err(|e| {
            match e {
            UnlockingError::WrongSecret { .. } => {
                Error::AdHoc(format!("{:?}", e).to_string())
            }
            UnlockingError::WrongKeyPair { .. } => {
                Error::AdHoc("exchange_success_public_key_hash was inconsistent with exchange_success_private_key".to_string())
            }
        }
        })?;

    let unlocking_parameters = htlc.unlock_with_secret(exchange_success_keypair.clone(), secret);

    let primed_txn = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            htlc_txid.clone().into(),
            vout,
            offer_created_event.btc_amount,
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
    //TODO: Store above in event prior to doing rnpc request
    let rpc_transaction = bitcoin_rpc::SerializedRawTransaction::from(redeem_tx);
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
