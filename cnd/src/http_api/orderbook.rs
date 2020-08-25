use crate::{
    asset::{
        Erc20Quantity, {self},
    },
    ethereum,
    http_api::{problem, serde_peer_id},
    network::BtcDaiOrderAddresses,
    Facade, Role,
};
use anyhow::Context;
use comit::{order, order::Denomination, BtcDaiOrderForm, OrderId, Position};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use time::NumericalDuration;
use warp::{http::StatusCode, reply, Rejection, Reply};

pub async fn post_make_btc_dai_order(
    body: MakeBtcDaiOrderBody,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
    let (form, addresses) = body.into_form_and_addresses();
    let id = facade.swarm.publish_order(form, addresses).await;

    Ok(reply::with_status(
        reply::with_header(warp::reply(), "Location", format!("/orders/{}", id)),
        StatusCode::CREATED,
    ))
}

pub async fn get_btc_dai_market(facade: Facade) -> Result<impl Reply, Rejection> {
    let entity = async {
        let mut orders = siren::Entity::default();
        let local_peer_id = facade.swarm.local_peer_id();

        for (maker, order) in facade.swarm.btc_dai_market().await {
            let market_item = siren::Entity::default()
                .with_properties(MarketItem {
                    id: order.id,
                    quantity: Amount::btc(order.quantity),
                    price: Amount::dai(order.price(Denomination::WeiPerBtc)),
                    ours: maker == local_peer_id,
                    maker,
                    position: order.position,
                })
                .context("failed to serialize market item sub entity")?;

            orders.push_sub_entity(siren::SubEntity::from_entity(market_item, &["item"]))
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
    price: Erc20Quantity,
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
        let swap_protocol =
            order::SwapProtocol::new(self.swap.role, position, 24.hours(), 12.hours()); // TODO: fill in good expiries here

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
struct OrderResponse {
    id: OrderId,
    position: Position,
    quantity: Amount,
    price: Amount,
}

#[derive(Clone, Debug, Serialize)]
struct MarketItem {
    id: OrderId,
    #[serde(with = "serde_peer_id")]
    maker: PeerId,
    ours: bool,
    position: Position,
    quantity: Amount,
    price: Amount,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "currency")]
enum Amount {
    #[serde(rename = "BTC")]
    Bitcoin {
        #[serde(with = "asset::bitcoin::sats_as_string")]
        value: asset::Bitcoin,
        decimals: u8,
    },
    #[serde(rename = "DAI")]
    Dai { value: Erc20Quantity, decimals: u8 },
}

impl Amount {
    fn btc(value: asset::Bitcoin) -> Self {
        Amount::Bitcoin { value, decimals: 8 }
    }

    fn dai(value: Erc20Quantity) -> Self {
        Amount::Dai {
            value,
            decimals: 18,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btc_amount_serializes_properly() {
        let amount = Amount::btc(asset::Bitcoin::from_sat(100000000));

        let string = serde_json::to_string(&amount).unwrap();

        assert_eq!(
            string,
            r#"{"currency":"BTC","value":"100000000","decimals":8}"#
        )
    }

    #[test]
    fn dai_amount_serializes_properly() {
        let amount =
            Amount::dai(Erc20Quantity::from_wei_dec_str("9000000000000000000000").unwrap());

        let string = serde_json::to_string(&amount).unwrap();

        assert_eq!(
            string,
            r#"{"currency":"DAI","value":"9000000000000000000000","decimals":18}"#
        )
    }
}
