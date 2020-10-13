use crate::{
    http_api::{problem, serde_peer_id, Amount},
    network::Swarm,
};
use anyhow::{Context, Result};
use comit::{expiries, order::SwapProtocol, BtcDaiOrder, OrderId, Position};
use futures::TryFutureExt;
use libp2p::PeerId;
use serde::Serialize;
use warp::{reply, Filter, Rejection, Reply};

/// The warp filter for getting the BTC/DAI market view.
pub fn route(
    swarm: Swarm,
    network: comit::Network,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path!("markets" / "BTC-DAI"))
        .and_then(move || {
            handler(swarm.clone(), network)
                .map_err(problem::from_anyhow)
                .map_err(warp::reject::custom)
        })
}

/// Retrieves viable orders: orders that have expiries that match the safe
/// expiries determined by the expiries module.
async fn handler(swarm: Swarm, network: comit::Network) -> Result<impl Reply> {
    let mut orders = siren::Entity::default();
    let local_peer_id = swarm.local_peer_id();

    let viable_orders = swarm
        .btc_dai_market()
        .await
        .into_iter()
        .filter(|(_, order)| has_viable_expiries(order, network));

    for (maker, order) in viable_orders {
        let market_item = siren::Entity::default()
            .with_properties(MarketItem {
                id: order.id,
                quantity: Amount::from(order.quantity),
                price: Amount::from(order.price),
                ours: maker == local_peer_id,
                maker,
                position: order.position,
            })
            .context("failed to serialize market item sub entity")?;

        orders.push_sub_entity(siren::SubEntity::from_entity(market_item, &["item"]))
    }

    Ok(reply::json(&orders))
}

pub fn has_viable_expiries(order: &BtcDaiOrder, network: comit::Network) -> bool {
    match order.swap_protocol {
        SwapProtocol::HbitHerc20 {
            hbit_expiry_offset,
            herc20_expiry_offset,
        } => {
            (hbit_expiry_offset, herc20_expiry_offset)
                == expiries::expiry_offsets_hbit_herc20(network)
        }
        SwapProtocol::Herc20Hbit {
            herc20_expiry_offset,
            hbit_expiry_offset,
        } => {
            (herc20_expiry_offset, hbit_expiry_offset)
                == expiries::expiry_offsets_herc20_hbit(network)
        }
    }
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

#[cfg(test)]
mod tests {
    use crate::http_api::markets::get_btc_dai::has_viable_expiries;
    use comit::{asset, order::SwapProtocol, BtcDaiOrder, Position, Price, Quantity, Role};
    use spectral::{assert_that, prelude::MappingIterAssertions};
    use time::Duration;

    #[test]
    fn filter_out_orders_with_unviable_expiries() {
        let order_with_viable_expiries = order_with_viable_expiries();
        let unfiltered_orders = vec![
            order_with_viable_expiries.clone(),
            order_with_unviable_expiries(),
        ];

        let filtered_orders = unfiltered_orders
            .into_iter()
            .filter(|order| has_viable_expiries(order, comit::Network::Dev))
            .collect::<Vec<BtcDaiOrder>>();

        assert_eq!(filtered_orders.len(), 1);
        assert_that(&filtered_orders)
            .matching_contains(|order| order_with_viable_expiries.id == order.id);
    }

    fn order_with_viable_expiries() -> BtcDaiOrder {
        BtcDaiOrder::sell(
            Quantity::new(asset::Bitcoin::ZERO),
            Price::from_wei_per_sat(asset::Erc20Quantity::zero()),
            SwapProtocol::new(Role::Alice, Position::Sell, comit::Network::Dev),
        )
    }

    fn order_with_unviable_expiries() -> BtcDaiOrder {
        let unsafe_hbit_expiry_offset = Duration::zero();
        let unsafe_herc20_expiry_offset = Duration::zero();

        assert_ne!(
            order_with_viable_expiries()
                .swap_protocol
                .hbit_expiry_offset(),
            unsafe_hbit_expiry_offset
        );
        assert_ne!(
            order_with_viable_expiries()
                .swap_protocol
                .herc20_expiry_offset(),
            unsafe_herc20_expiry_offset
        );

        BtcDaiOrder::sell(
            Quantity::new(asset::Bitcoin::ZERO),
            Price::from_wei_per_sat(asset::Erc20Quantity::zero()),
            SwapProtocol::HbitHerc20 {
                hbit_expiry_offset: unsafe_hbit_expiry_offset.into(),
                herc20_expiry_offset: unsafe_herc20_expiry_offset.into(),
            },
        )
    }
}
