use crate::{
    http_api::{amount::Amount, problem},
    Facade,
};
use anyhow::Result;
use comit::{asset::Erc20Quantity, OrderId, Position};
use futures::TryFutureExt;
use serde::{Serialize, Serializer};
use warp::{http::Method, Filter, Rejection, Reply};

/// The warp filter for getting a single order.
pub fn route(facade: Facade) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path!("orders" / OrderId))
        .and_then(move |order_id| {
            handler(order_id, facade.clone())
                .map_err(problem::from_anyhow)
                .map_err(warp::reject::custom)
        })
}

async fn handler(order_id: OrderId, facade: Facade) -> Result<impl Reply> {
    let db = &facade.storage.db;
    let (order, btc_dai_order) = db
        .do_in_transaction(|conn| {
            use crate::storage::tables::{BtcDaiOrder, Order};

            let order = Order::by_order_id(conn, order_id)?;
            let btc_dai_order = BtcDaiOrder::by_order(conn, &order)?;

            Ok((order, btc_dai_order))
        })
        .await?;

    let order = OrderResponse {
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
        )?,
    };

    let entity = make_entity(order)?;

    Ok(warp::reply::json(&entity))
}

fn make_entity(order: OrderResponse) -> Result<siren::Entity> {
    let order_id = order.id;
    let can_cancel = order.state.open > 0;

    let mut entity = siren::Entity::default().with_properties(order)?;

    if can_cancel {
        entity = entity.with_action(siren::Action {
            name: "cancel".to_string(),
            class: vec![],
            method: Some(Method::DELETE),
            href: format!("/orders/{}", order_id),
            title: None,
            _type: None,
            fields: vec![],
        });
    }

    Ok(entity)
}

/// The struct representing the properties within the siren document in our
/// response.
#[derive(Serialize)]
struct OrderResponse {
    id: OrderId,
    position: Position,
    price: Amount,
    quantity: Amount,
    state: State,
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
    fn new(open: i32, closed: i32, settling: i32, failed: i32, cancelled: i32) -> Result<Self> {
        Ok(Self {
            open: open as u8,
            closed: closed as u8,
            settling: settling as u8,
            failed: failed as u8,
            cancelled: cancelled as u8,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use comit::asset::{Bitcoin, Erc20Quantity};
    use uuid::Uuid;

    #[test]
    fn response_serializes_correctly() {
        let entity = make_entity(OrderResponse {
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
        })
        .unwrap();

        let result = serde_json::to_string_pretty(&entity).unwrap();

        assert_eq!(
            result,
            r#"{
  "class": [],
  "properties": {
    "id": "00000000-0000-0000-0000-000000000000",
    "position": "sell",
    "price": {
      "currency": "DAI",
      "decimals": 18,
      "value": "9100000000000000000000"
    },
    "quantity": {
      "currency": "BTC",
      "decimals": 8,
      "value": "10000000"
    },
    "state": {
      "cancelled": "0.00",
      "closed": "0.10",
      "failed": "0.60",
      "open": "0.30",
      "settling": "0.00"
    }
  },
  "entities": [],
  "links": [],
  "actions": [
    {
      "name": "cancel",
      "class": [],
      "method": "DELETE",
      "href": "/orders/00000000-0000-0000-0000-000000000000",
      "fields": []
    }
  ]
}"#
        );
    }
}
