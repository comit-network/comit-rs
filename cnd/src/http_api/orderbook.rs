use crate::{
    asset, hbit, herc20,
    http_api::problem,
    identity, ledger,
    network::NewOrder,
    storage::{CreatedSwap, Save},
    Facade, LocalSwapId, Role,
};
use chrono::Utc;
use comit::{
    ethereum,
    network::{Order, OrderId, SwapType},
};
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use warp::{http, http::StatusCode, Rejection, Reply};

#[derive(Deserialize)]
struct MakeHerc20HbitOrderBody {
    #[serde(with = "asset::bitcoin::sats_as_string")]
    btc_quantity: asset::Bitcoin,
    bitcoin_ledger: ledger::Bitcoin,
    erc20_token_contract: ethereum::Address,
    erc20_quantity: asset::Erc20Quantity,
    ethereum_ledger: ledger::Ethereum,
    alpha_expiry: u32,
    beta_expiry: u32,
    bob_refund_identity: bitcoin::Address,
    bob_redeem_identity: identity::Ethereum,
}

impl MakeHerc20HbitOrderBody {
    // TODO: This should implement From
    fn to_order(&self) -> NewOrder {
        NewOrder {
            btc_quantity: self.btc_quantity,
            bitcoin_ledger: self.bitcoin_ledger,
            erc20_quantity: asset::Erc20::new(
                self.erc20_token_contract,
                self.erc20_quantity.clone(),
            ),
            ethereum_ledger: self.ethereum_ledger,
            alpha_expiry: self.alpha_expiry,
            beta_expiry: self.beta_expiry,
        }
    }
}

#[derive(Deserialize)]
struct TakeHerc20HbitOrderBody {
    refund_identity: identity::Ethereum,
    redeem_identity: bitcoin::Address,
}

#[derive(Serialize)]
struct Herc20HbitOrderResponse {
    #[serde(with = "asset::bitcoin::sats_as_string")]
    btc_quantity: asset::Bitcoin,
    erc20_token_contract: ethereum::Address,
    erc20_quantity: asset::Erc20Quantity,
    alpha_expiry: u32,
    beta_expiry: u32,
    maker: String,
    id: OrderId,
}

impl Herc20HbitOrderResponse {
    // TODO: This should implement From
    fn from_order(order: &Order) -> Self {
        Herc20HbitOrderResponse {
            btc_quantity: order.btc_quantity,
            erc20_token_contract: order.erc20_quantity.token_contract,
            erc20_quantity: order.erc20_quantity.quantity.clone(),
            alpha_expiry: order.alpha_expiry,
            beta_expiry: order.beta_expiry,
            maker: order.maker.to_string(),
            id: order.id,
        }
    }
}

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
            asset: order.erc20_quantity,
            identity: refund_identity,
            chain_id: order.ethereum_ledger.chain_id,
            absolute_expiry: order.alpha_expiry,
        },
        beta: hbit::CreatedSwap {
            amount: asset::Bitcoin::from_sat(order.btc_quantity.as_sat()),
            final_identity: redeem_identity.clone(),
            network: ledger::Bitcoin::Regtest,
            absolute_expiry: order.beta_expiry,
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

// when making an order, the swap cannot be created until the take provides his
// identities. The swap is saved to the database when a TakeOrderRequest is
// received from the the taker.
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
    let order: NewOrder = body.to_order();

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
            body.bob_redeem_identity,
            body.bob_refund_identity.into(),
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

    facade
        .announce_trading_pair(::comit::network::TradingPair {
            buy: body.buy,
            sell: body.sell,
        })
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
            "erc20_token_contract": "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280",
            "bitcoin_ledger": "regtest",
            "erc20_quantity": "300",
            "btc_quantity": "200",
            "ethereum_ledger": {"chain_id":2},
            "alpha_expiry": 600,
            "beta_expiry": 300,
            "bob_refund_identity": "1F1tAaz5x1HUXrCNLbtMDqcw6o5GNn4xqX",
            "bob_redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
            "maker_addr": "/ip4/127.0.0.1/tcp/39331"
        }"#;

        let _body: MakeHerc20HbitOrderBody =
            serde_json::from_str(json).expect("failed to deserialize order");
    }
}
