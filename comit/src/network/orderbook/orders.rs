use crate::network::{
    orderbook::{Order, OrderId},
    NewOrder,
};
use libp2p::PeerId;
use std::collections::HashMap;

/// A collection of orders gathered from several makers.
#[derive(Clone, Debug)]
pub struct Orders {
    inner: HashMap<PeerId, HashMap<OrderId, Order>>,
    /// Our own id.
    ///
    /// Allows us to filter out our own orders.
    me: PeerId,
}

impl Orders {
    pub fn new(me: PeerId) -> Self {
        Self {
            inner: HashMap::default(),
            me,
        }
    }

    /// Get the peer id of the maker of this order.
    pub fn maker_id(&self, id: OrderId) -> Option<PeerId> {
        for (maker, orders) in self.inner.iter() {
            if orders.get(&id).is_some() {
                return Some(maker.clone());
            }
        }
        None
    }

    pub fn new_order(&mut self, form: NewOrder) -> OrderId {
        let id = OrderId::random();

        let order = Order {
            id,
            maker: self.me.clone(),
            position: form.position,
            bitcoin_amount: form.bitcoin_amount,
            bitcoin_ledger: form.bitcoin_ledger,
            bitcoin_absolute_expiry: form.bitcoin_absolute_expiry,
            price: form.price,
            token_contract: form.token_contract,
            ethereum_ledger: form.ethereum_ledger,
            ethereum_absolute_expiry: form.ethereum_absolute_expiry,
        };

        self.inner
            .entry(self.me.clone())
            .or_default()
            .insert(id, order);

        tracing::info!("created new order with id {}", id);

        id
    }

    /// Replaces the orders for this maker with the given list of orders.
    pub fn replace(&mut self, maker: PeerId, orders: Vec<Order>) {
        let mut map = HashMap::with_capacity(orders.len());
        for order in orders.into_iter() {
            map.insert(order.id, order);
        }

        self.inner.insert(maker, map);
    }

    pub fn remove_all_from(&mut self, maker: &PeerId) {
        self.inner.remove(maker);
    }

    pub fn clear_own_orders(&mut self) {
        self.inner.remove(&self.me);
    }

    pub fn cancel(&mut self, id: OrderId) {
        self.remove_ours(id);
    }

    pub fn remove_ours(&mut self, id: OrderId) -> Option<Order> {
        if let Some(map) = self.inner.get_mut(&self.me) {
            return map.remove(&id);
        }
        None
    }

    pub fn all(&self) -> impl Iterator<Item = &Order> {
        self.inner.values().flat_map(|orders| orders.values())
    }

    pub fn theirs(&self) -> impl Iterator<Item = &Order> {
        let me = &self.me;

        self.inner
            .iter()
            .filter_map(move |(maker, orders)| {
                if maker != me {
                    Some(orders.values())
                } else {
                    None
                }
            })
            .flatten()
    }

    pub fn ours(&self) -> impl Iterator<Item = &Order> {
        self.inner
            .get(&self.me)
            .map(|orders| orders.values())
            .into_iter()
            .flatten()
    }

    pub fn is_ours(&self, id: OrderId) -> bool {
        self.ours().any(|o| o.id == id)
    }
}
