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
    network::{MakerId, Order, OrderId, OrderNotFound, Position, Rate},
};
use serde::{Deserialize, Serialize};
use warp::{http, http::StatusCode, Rejection, Reply};

pub async fn post_take_order(
    order_id: OrderId,
    body: serde_json::Value,
    mut facade: Facade,
) -> Result<impl Reply, Rejection> {
    tracing::info!("entered take order controller");
    let body = TakeOrderBody::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let reply = warp::reply::reply();

    let swap_id = LocalSwapId::default();

    let order_id = order_id;
    let order = match facade.get_order(order_id).await {
        Some(order) => order,
        None => {
            return Err(OrderNotFound(order_id))
                .map_err(anyhow::Error::new)
                .map_err(problem::from_anyhow)
                .map_err(warp::reject::custom)
        }
    };

    // TODO: Consider putting the save in the network layer to be uniform with make?
    let start_of_swap = Utc::now().naive_local();

    let hbit = hbit::CreatedSwap {
        amount: body.bitcoin_amount,
        final_identity: body.bitcoin_identity.clone(),
        network: order.bitcoin_ledger,
        absolute_expiry: order.bitcoin_absolute_expiry,
    };

    let ethereum_amount = order
        .ethereum_amount(body.bitcoin_amount)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let herc20 = herc20::CreatedSwap {
        asset: Erc20 {
            token_contract: order.token_contract,
            quantity: ethereum_amount,
        },
        identity: body.ethereum_identity,
        chain_id: order.ethereum_ledger.chain_id,
        absolute_expiry: order.ethereum_absolute_expiry,
    };

    match order.position {
        Position::Buy => {
            let swap = CreatedSwap {
                swap_id,
                alpha: hbit,
                beta: herc20,
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
        }
        Position::Sell => {
            let swap = CreatedSwap {
                swap_id,
                alpha: herc20,
                beta: hbit,
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
        }
    }

    tracing::info!("swap created and saved from order: {:?}", order_id);

    facade
        .take_order(
            order_id,
            swap_id,
            body.bitcoin_identity.into(),
            body.ethereum_identity,
            body.bitcoin_amount,
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

pub async fn post_make_order(
    body: serde_json::Value,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
    tracing::info!("entered make order controller");
    let body = MakeOrderBody::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let reply = warp::reply::reply();
    let order = NewOrder::from(body.clone());

    order
        .assert_valid_ledger_pair()
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let swap_id = LocalSwapId::default();

    facade
        .make_order(
            order,
            swap_id,
            body.ethereum_identity,
            body.bitcoin_identity.into(),
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
        let bitcoin_field = siren::Field {
            name: "bitcoin_identity".to_string(),
            class: vec!["bitcoin".to_string(), "address".to_string()],
            _type: None,
            value: None,
            title: None,
        };

        let ethereum_field = siren::Field {
            name: "ethereum_identity".to_string(),
            class: vec!["ethereum".to_string(), "address".to_string()],
            _type: None,
            value: None,
            title: None,
        };

        let bitcoin_quantity = siren::Field {
            name: "bitcoin_amount".to_string(),
            class: vec!["bitcoin".to_string(), "quantity".to_string()],
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
            fields: vec![bitcoin_field, ethereum_field, bitcoin_quantity],
        };

        match siren::Entity::default()
            .with_action(action)
            .with_class_member("order")
            .with_properties(OrderResponse::from(order))
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
struct MakeOrderBody {
    position: Position,
    rate: Rate,
    #[serde(with = "asset::bitcoin::bitcoin_as_decimal_string")]
    bitcoin_amount: asset::Bitcoin,
    bitcoin_ledger: ledger::Bitcoin,
    bitcoin_absolute_expiry: u32,
    token_contract: identity::Ethereum,
    ethereum_ledger: ledger::Ethereum,
    ethereum_absolute_expiry: u32,
    bitcoin_identity: bitcoin::Address,
    ethereum_identity: identity::Ethereum,
}

impl From<MakeOrderBody> for NewOrder {
    fn from(body: MakeOrderBody) -> Self {
        NewOrder {
            position: body.position,
            rate: body.rate,
            bitcoin_ledger: body.bitcoin_ledger,
            bitcoin_absolute_expiry: body.bitcoin_absolute_expiry,
            bitcoin_amount: body.bitcoin_amount,
            token_contract: body.token_contract,
            ethereum_ledger: body.ethereum_ledger,
            ethereum_absolute_expiry: body.ethereum_absolute_expiry,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct TakeOrderBody {
    ethereum_identity: identity::Ethereum,
    bitcoin_identity: bitcoin::Address,
    #[serde(with = "asset::bitcoin::bitcoin_as_decimal_string")]
    bitcoin_amount: asset::Bitcoin,
}

#[derive(Clone, Debug, Serialize)]
struct OrderResponse {
    id: OrderId,
    maker: MakerId,
    position: Position,
    rate: Rate,
    bitcoin_ledger: ledger::Bitcoin,
    bitcoin_absolute_expiry: u32,
    #[serde(with = "asset::bitcoin::bitcoin_as_decimal_string")]
    bitcoin_amount: asset::Bitcoin,
    token_contract: ethereum::Address,
    ethereum_ledger: ledger::Ethereum,
    ethereum_absolute_expiry: u32,
}

impl From<Order> for OrderResponse {
    fn from(order: Order) -> Self {
        OrderResponse {
            id: order.id,
            maker: order.maker,
            position: order.position,
            rate: order.price,
            bitcoin_ledger: order.bitcoin_ledger,
            bitcoin_absolute_expiry: order.bitcoin_absolute_expiry,
            bitcoin_amount: order.bitcoin_quantity,
            token_contract: order.token_contract,
            ethereum_ledger: order.ethereum_ledger,
            ethereum_absolute_expiry: order.ethereum_absolute_expiry,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_order_deserialization() {
        let json = r#"
        {
            "position": "sell",
            "bitcoin_amount": "200.12",
            "bitcoin_ledger": "regtest",
            "bitcoin_absolute_expiry": 600,
            "rate": "9000.35",
            "token_contract": "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280",
            "ethereum_ledger": {"chain_id":2},
            "ethereum_absolute_expiry": 600,
            "bitcoin_identity": "1F1tAaz5x1HUXrCNLbtMDqcw6o5GNn4xqX",
            "ethereum_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72"
        }"#;

        let _body: MakeOrderBody = serde_json::from_str(json).expect("failed to deserialize order");
    }

    #[test]
    fn test_take_order_deserialization() {
        let json = r#"
        {
            "bitcoin_amount": "300.12",
            "bitcoin_identity": "1F1tAaz5x1HUXrCNLbtMDqcw6o5GNn4xqX",
            "ethereum_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72"
        }"#;

        let _body: TakeOrderBody = serde_json::from_str(json).expect("failed to deserialize order");
    }
}
