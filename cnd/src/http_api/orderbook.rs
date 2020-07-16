use crate::{
    asset::{self, Erc20},
    hbit, herc20,
    http_api::problem,
    identity, ledger,
    network::NewOrder,
    storage::{CreatedSwap, Save},
    Facade, LocalSwapId, Role,
};
use chrono::Utc;
use comit::{
    ethereum,
    network::{MakerId, Order, OrderId, Position},
};
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use warp::{http, http::StatusCode, Rejection, Reply};

pub async fn post_take_herc20_hbit_order(
    order_id: OrderId,
    body: serde_json::Value,
    mut facade: Facade,
) -> Result<impl Reply, Rejection> {
    tracing::info!("entered take order controller");
    let body = TakeHerc20HbitOrderBody::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let reply = warp::reply::reply();

    let refund_identity = body.refund_identity;
    let redeem_identity = body.redeem_identity;

    let swap_id = LocalSwapId::default();

    let order_id = order_id;
    let order = match facade.get_order(order_id).await {
        Some(order) => order,
        None => panic!("order not found"),
    };

    // TODO: Consider putting the save in the network layer to be uniform with make?

    let start_of_swap = Utc::now().naive_local();

    let swap = CreatedSwap {
        swap_id,
        alpha: herc20::CreatedSwap {
            asset: Erc20 {
                token_contract: order.token_contract,
                quantity: order.ethereum_amount,
            },
            identity: refund_identity,
            chain_id: order.ethereum_ledger.chain_id,
            absolute_expiry: order.ethereum_absolute_expiry,
        },
        beta: hbit::CreatedSwap {
            amount: order.bitcoin_amount,
            final_identity: redeem_identity.clone(),
            network: order.bitcoin_ledger,
            absolute_expiry: order.bitcoin_absolute_expiry,
        },
        peer: order.maker.clone().into(),
        address_hint: None,
        role: Role::Alice,
        start_of_swap,
    };

    facade
        .save(swap)
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    tracing::info!("swap created and saved from order: {:?}", order_id);

    facade
        .take_herc20_hbit_order(order_id, swap_id, redeem_identity.into(), refund_identity)
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

pub async fn post_make_herc20_hbit_order(
    body: serde_json::Value,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
    tracing::info!("entered take order controller");
    let body = MakeHerc20HbitOrderBody::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let reply = warp::reply::reply();
    let order = NewOrder::from(body.clone());

    order
        .assert_valid_ledger_pair()
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    // TODO: We need to save the bitcoin address here else it is lost.
    let swap_id = LocalSwapId::default();

    facade
        .make_herc20_hbit_order(
            order,
            swap_id,
            body.redeem_identity,
            body.refund_identity.into(),
        )
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
            .with_properties(Herc20HbitOrderResponse::from(order))
        {
            Ok(sub_entity) => {
                entity.push_sub_entity(siren::SubEntity::from_entity(sub_entity, &["item"]))
            }
            Err(_e) => tracing::error!("could not serialise order sub entity"),
        }
    }
    Ok(warp::reply::json(&entity))
}

#[derive(Clone, Debug, Deserialize)]
struct MakeHerc20HbitOrderBody {
    position: Position,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    bitcoin_amount: asset::Bitcoin,
    bitcoin_ledger: ledger::Bitcoin,
    bitcoin_absolute_expiry: u32,
    ethereum_amount: asset::Erc20Quantity,
    token_contract: identity::Ethereum,
    ethereum_ledger: ledger::Ethereum,
    ethereum_absolute_expiry: u32,
    refund_identity: bitcoin::Address,
    redeem_identity: identity::Ethereum,
}

impl From<MakeHerc20HbitOrderBody> for NewOrder {
    fn from(body: MakeHerc20HbitOrderBody) -> Self {
        NewOrder {
            position: body.position,
            bitcoin_amount: body.bitcoin_amount,
            bitcoin_ledger: body.bitcoin_ledger,
            bitcoin_absolute_expiry: body.bitcoin_absolute_expiry,
            ethereum_amount: body.ethereum_amount,
            token_contract: body.token_contract,
            ethereum_ledger: body.ethereum_ledger,
            ethereum_absolute_expiry: body.ethereum_absolute_expiry,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct TakeHerc20HbitOrderBody {
    refund_identity: identity::Ethereum,
    redeem_identity: bitcoin::Address,
}

#[derive(Clone, Debug, Serialize)]
struct Herc20HbitOrderResponse {
    id: OrderId,
    maker: MakerId,
    position: Position,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    bitcoin_amount: asset::Bitcoin,
    bitcoin_ledger: ledger::Bitcoin,
    bitcoin_absolute_expiry: u32,
    ethereum_amount: asset::Erc20Quantity,
    token_contract: ethereum::Address,
    ethereum_ledger: ledger::Ethereum,
    ethereum_absolute_expiry: u32,
}

impl From<Order> for Herc20HbitOrderResponse {
    fn from(order: Order) -> Self {
        Herc20HbitOrderResponse {
            id: order.id,
            maker: order.maker,
            position: order.position,
            bitcoin_amount: order.bitcoin_amount,
            bitcoin_ledger: order.bitcoin_ledger,
            bitcoin_absolute_expiry: order.bitcoin_absolute_expiry,
            ethereum_amount: order.ethereum_amount,
            token_contract: order.token_contract,
            ethereum_ledger: order.ethereum_ledger,
            ethereum_absolute_expiry: order.ethereum_absolute_expiry,
        }
    }
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

// TODO: Add ser stability and roundtrip tests.
#[derive(Deserialize, Debug, Copy, Clone)]
pub enum TradingPair {
    BtcDai,
}

impl From<TradingPair> for comit::network::orderbook::TradingPair {
    fn from(tp: TradingPair) -> Self {
        match tp {
            TradingPair::BtcDai => comit::network::orderbook::TradingPair::BtcDai,
        }
    }
}

pub async fn post_announce_trading_pair(
    body: serde_json::Value,
    mut facade: Facade,
) -> Result<impl Reply, Rejection> {
    let tp = TradingPair::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    facade
        .announce_trading_pair(tp.into())
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    Ok(warp::reply::reply())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_order_deserialization() {
        let json = r#"
        {
            "position": "sell",
            "bitcoin_amount": "300",
            "bitcoin_ledger": "regtest",
            "bitcoin_absolute_expiry": 600,
            "ethereum_amount": "200",
            "token_contract": "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280",
            "ethereum_ledger": {"chain_id":2},
            "ethereum_absolute_expiry": 600,
            "refund_identity": "1F1tAaz5x1HUXrCNLbtMDqcw6o5GNn4xqX",
            "redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72"
        }"#;

        let _body: MakeHerc20HbitOrderBody =
            serde_json::from_str(json).expect("failed to deserialize order");
    }
}
