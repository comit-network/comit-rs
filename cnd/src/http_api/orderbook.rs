use crate::{
    asset,
    ethereum::ChainId,
    hbit, herc20,
    http_api::problem,
    identity, ledger,
    storage::{CreatedSwap, Save},
    Facade, LocalSwapId, Role, Side,
};
use chrono::Utc;
use comit::{
    ethereum,
    network::{NewOrder, Order, OrderId, SwapType},
};
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::{http, http::StatusCode, Rejection, Reply};

#[derive(Deserialize)]
struct MakeHerc20HbitOrderBody {
    pub buy_quantity: asset::Bitcoin,
    pub sell_token_contract: ethereum::Address,
    pub sell_quantity: asset::Erc20Quantity,
    pub absolute_expiry: u32,
    pub refund_identity: bitcoin::Address,
    pub redeem_identity: identity::Ethereum,
    pub maker_addr: Multiaddr,
}

impl MakeHerc20HbitOrderBody {
    fn to_order(&self) -> NewOrder {
        NewOrder {
            buy: self.buy_quantity,
            sell: asset::Erc20::new(self.sell_token_contract, self.sell_quantity.clone()),
            absolute_expiry: self.absolute_expiry,
            maker_addr: self.maker_addr.clone(),
        }
    }
}

#[derive(Deserialize)]
struct TakeHbitHerc20OrderBody {
    refund_identity: bitcoin::Address,
    redeem_identity: identity::Ethereum,
}

#[derive(Serialize)]
struct Herc20HbitOrderResponse {
    pub buy_quantity: asset::Bitcoin,
    pub sell_token_contract: ethereum::Address,
    pub sell_quantity: asset::Erc20Quantity,
    pub absolute_expiry: u32,
    pub maker: String,
    pub id: Uuid,
}

impl Herc20HbitOrderResponse {
    pub fn from_order(order: &Order) -> Self {
        Herc20HbitOrderResponse {
            buy_quantity: asset::Bitcoin::from_sat(order.buy),
            sell_token_contract: order.sell.token_contract,
            sell_quantity: order.sell.quantity.clone(),
            absolute_expiry: order.absolute_expiry,
            maker: PeerId::from_bytes(order.maker.clone()).unwrap().to_string(),
            id: order.id,
        }
    }
}

pub async fn post_take_hbit_herc20_order(
    order_id: OrderId,
    body: serde_json::Value,
    mut facade: Facade,
) -> Result<impl Reply, Rejection> {
    tracing::info!("entered take order controller");
    let body = TakeHbitHerc20OrderBody::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let reply = warp::reply::reply();

    let refund_identity = body.refund_identity.clone();
    let redeem_identity = body.redeem_identity;

    let swap_id = LocalSwapId::default();

    let order_id = order_id;
    let order = match facade.get_order(order_id).await {
        Some(order) => order,
        None => panic!("order not found"),
    };

    let start_of_swap = Utc::now().naive_local();

    let swap = CreatedSwap {
        swap_id,
        alpha: hbit::CreatedSwap {
            amount: asset::Bitcoin::from_sat(order.buy),
            final_identity: refund_identity.clone(),
            network: ledger::Bitcoin::Regtest,
            absolute_expiry: order.absolute_expiry,
        },
        beta: herc20::CreatedSwap {
            asset: order.sell,
            identity: redeem_identity,
            chain_id: ChainId::regtest(),
            absolute_expiry: order.absolute_expiry,
        },
        // todo: remove the unwrap
        peer: PeerId::from_bytes(order.maker).unwrap(),
        address_hint: Some(order.maker_addr),
        role: Role::Bob,
        start_of_swap,
    };

    facade
        .save(swap)
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    tracing::info!("swap created and saved from order: {:?}", order_id);

    let transient_key = facade
        .storage
        .derive_transient_identity(swap_id, Role::Bob, Side::Alpha);

    tracing::info!("derivied transient key for swap: {}", swap_id);

    facade
        .take_hbit_herc20_order(
            order_id,
            swap_id,
            refund_identity.clone().into(),
            transient_key,
            redeem_identity,
        )
        .await
        .map(|_| {
            warp::reply::with_status(
                warp::reply::with_header(reply, "Location", format!("/swaps/{}", swap_id)),
                StatusCode::CREATED,
            )
        })
        // do error handling on in from_anyhow
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

// when making an order, the swap cannot be created until the take provides his
// identities. The swap is saved to the database when a TakeOrderRequest is
// received from the the taker.
pub async fn post_make_hbit_herc20_order(
    body: serde_json::Value,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
    tracing::info!("entered take order controller");
    let body = MakeHerc20HbitOrderBody::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let reply = warp::reply::reply();
    let order: NewOrder = body.to_order();

    facade
        .make_hbit_herc20_order(order, body.refund_identity.into(), body.redeem_identity)
        .await
        .map(|order_id| {
            warp::reply::with_status(
                warp::reply::with_header(reply, "Location", format!("/orders/{}", order_id)),
                StatusCode::CREATED,
            )
        })
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

pub async fn get_order(order_id: OrderId, facade: Facade) -> Result<impl Reply, Rejection> {
    let swap_id = facade
        .storage
        .get_swap_associated_with_order(&order_id)
        .await;

    // let entity = siren::Entity::default().with_class_member("order");
    let entity = match swap_id {
        Some(swap_id) => siren::Entity::default()
            .with_class_member("order")
            .with_link(
                siren::NavigationalLink::new(&["swap"], format!("/swaps/{}", swap_id))
                    .with_title("swap that was created from the order"),
            ),
        None => siren::Entity::default().with_class_member("order"),
    };
    Ok(warp::reply::json(&entity))
}

pub async fn get_orders(facade: Facade) -> Result<impl Reply, Rejection> {
    let orders = facade.get_orders().await;

    let mut entity = siren::Entity::default().with_class_member("orders");

    for order in orders.into_iter() {
        let redeem_field = siren::Field {
            name: "redeem_identity".to_string(),
            class: vec!["ethereum".to_string(), "address".to_string()],
            _type: None,
            value: None,
            title: None,
        };

        let refund_field = siren::Field {
            name: "refund_identity".to_string(),
            class: vec!["bitcoin".to_string(), "address".to_string()],
            _type: None,
            value: None,
            title: None,
        };

        let action = siren::Action {
            name: "take".to_string(),
            class: vec![],
            method: Some(http::Method::POST),
            href: format!("/orders/{}/take", order.id),
            title: None,
            _type: Some("application/json".to_string()),
            fields: vec![redeem_field, refund_field],
        };

        match siren::Entity::default()
            .with_action(action)
            .with_class_member("order")
            .with_properties(Herc20HbitOrderResponse::from_order(&order.clone()))
        {
            Ok(sub_entity) => {
                entity.push_sub_entity(siren::SubEntity::from_entity(sub_entity, &["item"]))
            }
            Err(_e) => tracing::error!("could not serialise order sub entity"),
        }
    }
    Ok(warp::reply::json(&entity))
}

#[derive(Deserialize, Debug)]
pub struct DialPeerBody {
    addresses: Vec<Multiaddr>,
}

pub async fn post_dial_peer(
    body: serde_json::Value,
    mut facade: Facade,
) -> Result<impl Reply, Rejection> {
    let body = DialPeerBody::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;
    // todo: find out if the dial sucessful?
    for addr in body.addresses {
        facade.dial_addr(addr).await;
    }
    Ok(warp::reply::reply())
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct TradingPair {
    buy: SwapType,
    sell: SwapType,
}

pub async fn post_announce_trading_pair(
    body: serde_json::Value,
    mut facade: Facade,
) -> Result<impl Reply, Rejection> {
    let body = TradingPair::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;
    // todo: find out if the dial sucessful?

    facade
        .announce_trading_pair(::comit::network::TradingPair {
            buy: body.buy,
            sell: body.sell,
        })
        .await;

    Ok(warp::reply::reply())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_order_deserialization() {
        let json = r#"
        {
            "sell_token_contract": "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280",
            "buy_quantity": "300",
            "sell_quantity": "200",
            "absolute_expiry": 600,
            "refund_identity": "1F1tAaz5x1HUXrCNLbtMDqcw6o5GNn4xqX",
            "redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
            "maker_addr": "/ip4/127.0.0.1/tcp/39331"
        }"#;

        let _body: MakeHerc20HbitOrderBody = serde_json::from_str(json).unwrap();
    }
}
