use crate::{
    asset::{self},
    ethereum,
    http_api::{problem, serde_peer_id},
    network::BtcDaiOrderAddresses,
    Facade, Role,
};
use anyhow::Context;
use comit::{order, BtcDaiOrderForm, OrderId, Position};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use time::Duration;
use warp::{http::StatusCode, reply, Rejection, Reply};

pub async fn post_make_btc_dai_order(body: MakeBtcDaiOrderBody, facade: Facade) -> impl Reply {
    let (form, addresses) = body.into_form_and_addresses();
    let id = facade.swarm.publish_order(form, addresses).await;

    reply::with_status(
        reply::with_header(warp::reply(), "Location", format!("/orders/{}", id)),
        StatusCode::CREATED,
    )
}

pub async fn get_orders(facade: Facade) -> Result<impl Reply, Rejection> {
    let entity = async {
        let mut orders = siren::Entity::default().with_class_member("orders");
        let local_peer_id = facade.swarm.local_peer_id();

        for (maker, order) in facade.get_orders().await {
            let order = siren::Entity::default()
                .with_class_member("order")
                .with_properties(BtcDaiOrderResponse {
                    id: order.id,
                    ours: maker == local_peer_id,
                    maker,
                    position: order.position,
                    quantity: order.quantity,
                    price: order.price,
                    trading_pair: TradingPair::BtcDai,
                })
                .context("failed to serialize order sub entity")?;

            orders.push_sub_entity(siren::SubEntity::from_entity(order, &["item"]))
        }

        Ok(orders)
    }
    .await
    .map_err(problem::from_anyhow)
    .map_err(warp::reject::custom)?;

    Ok(reply::json(&entity))
}

#[derive(Debug, Deserialize)]
pub struct MakeBtcDaiOrderBody {
    position: Position,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    quantity: asset::Bitcoin,
    price: u64,
    swap: SwapParams,
}

#[derive(Debug, Deserialize)]
pub struct SwapParams {
    #[serde(default = "default_role")]
    role: Role,
    bitcoin_address: bitcoin::Address,
    ethereum_address: ethereum::Address,
}

fn default_role() -> Role {
    Role::Alice
}

impl MakeBtcDaiOrderBody {
    fn into_form_and_addresses(self) -> (BtcDaiOrderForm, BtcDaiOrderAddresses) {
        let position = self.position;
        let swap_protocol = order::SwapProtocol::new(
            self.swap.role,
            position,
            Duration::default(),
            Duration::default(),
        ); // TODO: fill in good expiries here

        let form = BtcDaiOrderForm {
            position,
            quantity: self.quantity,
            price: self.price,
            swap_protocol,
        };
        let addresses = BtcDaiOrderAddresses {
            bitcoin: self.swap.bitcoin_address,
            ethereum: self.swap.ethereum_address,
        };

        (form, addresses)
    }
}

#[derive(Clone, Debug, Serialize)]
struct BtcDaiOrderResponse {
    id: OrderId,
    #[serde(with = "serde_peer_id")]
    maker: PeerId,
    ours: bool,
    position: Position,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    quantity: asset::Bitcoin,
    price: u64,
    trading_pair: TradingPair,
}

#[derive(Clone, Debug, Serialize)]
enum TradingPair {
    #[serde(rename = "BTC/DAI")]
    BtcDai,
}

#[cfg(test)]
mod tests {

    #[test]
    fn deserialize_make_order_body() {
        unimplemented!()
    }
}
