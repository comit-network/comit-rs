mod cancel;
mod get_single;
mod list_open;
mod make_btc_dai;

pub use cancel::route as cancel;
pub use get_single::route as get_single;
pub use list_open::route as list_open;
pub use make_btc_dai::route as make_btc_dai;

use crate::{
    http_api::amount::Amount,
    storage::{tables, BtcDaiOrder, Order},
};
use anyhow::Result;
use comit::{asset::Erc20Quantity, OrderId, Position};
use serde::{Serialize, Serializer};
use warp::http::Method;

/// The struct representing the properties within the siren document in our
/// response.
#[derive(Serialize)]
struct OrderProperties {
    id: OrderId,
    position: Position,
    price: Amount,
    quantity: Amount,
    state: State,
}

impl From<(tables::Order, tables::BtcDaiOrder)> for OrderProperties {
    fn from(tuple: (Order, BtcDaiOrder)) -> Self {
        let (order, btc_dai_order) = tuple;

        Self {
            id: order.order_id.0,
            position: order.position.0,
            price: Amount::dai(100_000_000 * Erc20Quantity::from(btc_dai_order.price.0)), /* TODO: Consolidate this with logic in BtcDaiOrder model */
            quantity: Amount::btc(btc_dai_order.quantity.0.into()),
            state: State::new(
                order.open,
                order.closed,
                order.settling,
                order.failed,
                order.cancelled,
            ),
        }
    }
}

#[derive(Serialize)]
struct State {
    #[serde(serialize_with = "percent_string")]
    open: u8,
    #[serde(serialize_with = "percent_string")]
    closed: u8,
    #[serde(serialize_with = "percent_string")]
    settling: u8,
    #[serde(serialize_with = "percent_string")]
    failed: u8,
    #[serde(serialize_with = "percent_string")]
    cancelled: u8,
}

impl State {
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // we only store positive values in the DB ranging from 0 - 100
    fn new(open: i32, closed: i32, settling: i32, failed: i32, cancelled: i32) -> Self {
        Self {
            open: open as u8,
            closed: closed as u8,
            settling: settling as u8,
            failed: failed as u8,
            cancelled: cancelled as u8,
        }
    }
}

fn percent_string<S>(value: &u8, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    #[allow(clippy::cast_precision_loss)] // we only deal with very small values here (0 - 100)
    let percent = (*value as f32) / 100f32;

    serializer.serialize_str(&format!("{:.2}", percent))
}

fn make_order_entity(properties: OrderProperties) -> Result<siren::Entity> {
    let mut entity = siren::Entity::default().with_properties(&properties)?;

    if let Some(action) = cancel_action(&properties) {
        entity = entity.with_action(action)
    }

    Ok(entity)
}

fn cancel_action(order: &OrderProperties) -> Option<siren::Action> {
    if order.state.open > 0 {
        Some(siren::Action {
            name: "cancel".to_string(),
            class: vec![],
            method: Some(Method::DELETE),
            href: format!("/orders/{}", order.id),
            title: None,
            _type: Some("application/json".to_owned()),
            fields: vec![],
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use comit::asset::{Bitcoin, Erc20Quantity};
    use uuid::Uuid;

    #[test]
    fn response_serializes_correctly() {
        let properties = OrderProperties {
            id: OrderId::from(Uuid::from_u128(0)),
            position: Position::Sell,
            price: Amount::dai(Erc20Quantity::from_wei_dec_str("9100000000000000000000").unwrap()),
            quantity: Amount::btc(Bitcoin::from_sat(10000000)),
            state: State {
                open: 30,
                closed: 10,
                settling: 0,
                failed: 60,
                cancelled: 0,
            },
        };

        let result = serde_json::to_string_pretty(&properties).unwrap();

        assert_eq!(
            result,
            r#"{
  "id": "00000000-0000-0000-0000-000000000000",
  "position": "sell",
  "price": {
    "currency": "DAI",
    "value": "9100000000000000000000",
    "decimals": 18
  },
  "quantity": {
    "currency": "BTC",
    "value": "10000000",
    "decimals": 8
  },
  "state": {
    "open": "0.30",
    "closed": "0.10",
    "settling": "0.00",
    "failed": "0.60",
    "cancelled": "0.00"
  }
}"#
        );
    }
}
