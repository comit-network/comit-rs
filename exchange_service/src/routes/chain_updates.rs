extern crate common_types;
extern crate bitcoin_rpc;
extern crate bitcoin_wallet;

use common_types::Secret;
use bitcoin_rpc;
use bitcoin_wallet;

#[derive(Deserialize)]
pub struct RedeemBTCNotificationBody {
    secret: Secret
}

#[post("/trades/ETH-BTC/<trade_id>/buy-order-secret-revealed", format = "application/json", data = "<redeem_btc_notification_body>")]
pub fn post_revealed_secret(
    redeem_btc_notification_body: Json<RedeemBTCNotificationBody>,
    event_store: State<EventStore>,
) -> Result<(), BadRequest<String>> {
    let htlc_tx_id: bitcoin_rpc::TransactionId = unimplemented!();


    bitcoin_wallet:
}

