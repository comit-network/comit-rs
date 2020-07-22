//! Abstract Data Type for managing the orders contained in the orderbook.

use crate::network::protocols::orderbook::{BtcDaiOrder, OrderId};
use anyhow::bail;
use libp2p::PeerId;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Orders {
    inner: HashMap<OrderId, Order>,
}

impl Orders {
    /// Insert a new live order. A BTC/DAI order is immutable, it is therefore
    /// an error to insert the same order twice.
    pub fn insert(&mut self, maker: PeerId, order: BtcDaiOrder) -> anyhow::Result<()> {
        let id = order.id;

        let r = Order {
            order,
            maker,
            status: Status::Live,
        };

        if self.inner.get(&id).is_some() {
            bail!("order already exists: {}", id);
        }
        let _ = self.inner.insert(id, r);

        Ok(())
    }

    /// Returns the peer id of the node that created this order.
    pub fn maker(&self, id: &OrderId) -> Option<PeerId> {
        self.inner.get(id).map(|r| r.maker.clone())
    }

    /// Returns true if an order with `id` is known.
    pub fn contains(&self, id: &OrderId) -> bool {
        self.inner.contains_key(id)
    }

    /// Returns true if the order is live i.e., has been seen by the network and
    /// a delete message has not been seen. See [Status] for more details.
    pub fn is_live(&self, id: &OrderId) -> bool {
        match self.inner.get(id) {
            None => false,
            Some(r) => r.status == Status::Live,
        }
    }

    /// Gets all orders, live and dead.
    pub fn get_orders(&self) -> Vec<BtcDaiOrder> {
        self.inner.values().map(|r| r.order).collect()
    }

    /// Gets a specific order if it is known.
    pub fn get_order(&self, id: &OrderId) -> Option<BtcDaiOrder> {
        self.inner.get(id).map(|r| r.order)
    }

    /// Kill an order. Return true if order was killed, false if order is
    /// already dead, and an error if order does not exist.
    pub fn kill_order(&mut self, maker: PeerId, id: &OrderId) -> anyhow::Result<bool> {
        let update = match self.inner.get_mut(id) {
            None => bail!("order not found"),
            Some(mut r) => {
                if r.maker != maker {
                    bail!("cannot kill someone else's order");
                }
                let update = r.status == Status::Live;
                r.status = Status::Dead;
                update
            }
        };

        Ok(update)
    }
}

impl Default for Orders {
    fn default() -> Self {
        Orders {
            inner: HashMap::new(),
        }
    }
}

/// Conceptually orders are 'owned' by the peer that creates them (the maker),
/// only the maker can gossip create/delete for this order id so we associate
/// the makers peer id with the order.
#[derive(Debug, PartialEq)]
struct Order {
    order: BtcDaiOrder,
    maker: PeerId,
    status: Status,
}

/// We purposely do not use the terms filled/cancelled because the network
/// has no way of guaranteeing these things, from the networks perspective
/// an order is 'live' if it has been received and 'dead' if a the order was
/// deleted by the node that created it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    Live,
    Dead, // Cancelled or filled.
}
