//! This file contains most of the logic for making new orders through the HTTP
//! API.
//!
//! Compared to other routes, emphasis has been placed on making this file
//! self-contained by exposing a single filter and keeping every thing else
//! private.

use crate::{
    asset::{
        Erc20Quantity, {self},
    },
    config::Settings,
    ethereum,
    http_api::problem,
    storage::{
        InsertableBtcDaiOrder, InsertableOrder, InsertableOrderHbitParams,
        InsertableOrderHerc20Params,
    },
    Facade, Role,
};
use anyhow::Result;
use comit::{
    order::{Denomination, SwapProtocol},
    BtcDaiOrder, Position, Side,
};
use diesel::SqliteConnection;
use futures::TryFutureExt;
use serde::Deserialize;
use warp::{http::StatusCode, Filter, Rejection, Reply};

/// The warp filter for making a new BTC/DAI order.
pub fn route(
    facade: Facade,
    settings: Settings,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path!("orders" / "BTC-DAI"))
        .and(warp::body::json())
        .and_then(move |body| {
            handler(body, facade.clone(), settings.clone())
                .map_err(problem::from_anyhow)
                .map_err(warp::reject::custom)
        })
}

async fn handler(body: Body, facade: Facade, settings: Settings) -> Result<impl Reply> {
    let db = facade.storage.db;
    let swarm = facade.swarm;

    let order = BtcDaiOrder::new(
        body.position,
        body.quantity,
        body.price,
        SwapProtocol::new(body.swap.role, body.position),
    );
    let order_id = order.id;

    db.do_in_transaction(save_order(order.clone(), body.swap, settings))
        .await?;
    swarm.publish_order(order).await;

    Ok(warp::reply::with_header(
        StatusCode::CREATED,
        "Location",
        format!("/orders/{}", order_id),
    ))
}

#[derive(Debug, Deserialize)]
struct Body {
    position: Position,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    quantity: asset::Bitcoin,
    price: Erc20Quantity,
    swap: SwapParams,
}

#[derive(Debug, Deserialize)]
struct SwapParams {
    #[serde(default = "default_role")]
    role: Role,
    bitcoin_address: bitcoin::Address,
    ethereum_address: ethereum::Address,
}

fn default_role() -> Role {
    Role::Alice
}

fn save_order(
    order: BtcDaiOrder,
    swap: SwapParams,
    settings: Settings,
) -> impl FnOnce(&SqliteConnection) -> Result<()> {
    let insertable_order = InsertableOrder::new(order.id, order.position, order.created_at);

    let insertable_btc_dai_order = {
        let quantity = order.quantity;
        let price = order.price(Denomination::WeiPerSat);

        move |order_fk| InsertableBtcDaiOrder::new(order_fk, quantity, price)
    };

    let insertable_hbit = {
        let network = settings.bitcoin.network;
        let swap_protocol = order.swap_protocol;
        let our_final_address = swap.bitcoin_address;

        move |order_fk| {
            InsertableOrderHbitParams::new(
                order_fk,
                network,
                our_final_address,
                swap_protocol.hbit_expiry_offset().whole_seconds(),
                match swap_protocol {
                    SwapProtocol::HbitHerc20 { .. } => Side::Alpha,
                    SwapProtocol::Herc20Hbit { .. } => Side::Beta,
                },
            )
        }
    };

    let insertable_herc20_params = {
        let chain_id = settings.ethereum.chain_id;
        let dai_contract = settings.ethereum.tokens.dai;
        let swap_protocol = order.swap_protocol;
        let our_htlc_identity = swap.ethereum_address;

        move |order_fk| {
            InsertableOrderHerc20Params::new(
                order_fk,
                chain_id,
                our_htlc_identity,
                dai_contract,
                swap_protocol.herc20_expiry_offset().whole_seconds(),
                match swap_protocol {
                    SwapProtocol::Herc20Hbit { .. } => Side::Alpha,
                    SwapProtocol::HbitHerc20 { .. } => Side::Beta,
                },
            )
        }
    };

    move |conn| {
        let order_fk = insertable_order.insert(conn)?;

        insertable_btc_dai_order(order_fk).insert(conn)?;
        insertable_hbit(order_fk).insert(conn)?;
        insertable_herc20_params(order_fk).insert(conn)?;

        Ok(())
    }
}
