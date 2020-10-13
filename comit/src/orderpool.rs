use crate::{
    asset, asset::Erc20Quantity, order::SwapProtocol, BtcDaiOrder, OrderId, Position, Price,
    Quantity,
};
use anyhow::Result;
use libp2p::PeerId;
use lru::LruCache;
use std::{
    collections::{hash_map::Entry, HashMap},
    iter,
    iter::FromIterator,
    ops::AddAssign,
};
use time::OffsetDateTime;

/// A collection of orders gathered from several makers.
#[derive(Debug)]
pub struct OrderPool {
    inner: HashMap<PeerId, HashMap<OrderId, BtcDaiOrder>>,

    reserved_quantities: HashMap<OrderId, asset::Bitcoin>,
    /// Our own id.
    ///
    /// Allows us to filter out our own orders.
    me: PeerId,

    /// A cache for storing which orders don't match.
    no_match_cache: LruCache<NoMatch, ()>,
}

/// Serves as the key in our no-match cache.
///
/// Using both orders for the key has two advantages:
///
/// 1. We don't consume as much memory because only the hash of this struct is
/// stored. 2. It allows our cache to automatically invalidate itself if the
/// quantity of any of the orders change.
#[derive(Debug, PartialEq, Eq, Hash)]
struct NoMatch {
    ours: BtcDaiOrder,
    theirs: BtcDaiOrder,
}

impl OrderPool {
    pub fn new(me: PeerId) -> Self {
        Self {
            inner: Default::default(),
            reserved_quantities: Default::default(),
            me,
            no_match_cache: LruCache::new(100), /* cap this at a 100 entries to avoid unbounded
                                                 * memory growth */
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

    pub fn publish(&mut self, order: BtcDaiOrder) {
        let id = order.id;
        self.inner
            .entry(self.me.clone())
            .or_default()
            .insert(id, order);

        tracing::info!("published order {}", id);
    }

    /// Receive other people's orders.
    ///
    /// This replaces all current orders of this peer with the newly received
    /// ones.
    pub fn receive(&mut self, maker: PeerId, orders: Vec<BtcDaiOrder>) {
        let map = HashMap::from_iter(orders.into_iter().map(|o| (o.id, o)));

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

    pub fn remove_ours(&mut self, id: OrderId) -> Option<BtcDaiOrder> {
        if let Some(map) = self.inner.get_mut(&self.me) {
            return map.remove(&id);
        }
        None
    }

    pub fn all(&self) -> impl Iterator<Item = (&PeerId, &BtcDaiOrder)> {
        self.inner
            .iter()
            .flat_map(|(maker, orders)| iter::from_fn(move || Some(maker)).zip(orders.values()))
    }

    pub fn theirs(&self) -> impl Iterator<Item = (&PeerId, &BtcDaiOrder)> + Clone {
        let me = &self.me;

        self.inner
            .iter()
            .filter_map(move |(maker, orders)| {
                if maker != me {
                    Some(iter::from_fn(move || Some(maker)).zip(orders.values()))
                } else {
                    None
                }
            })
            .flatten()
    }

    pub fn ours(&self) -> impl Iterator<Item = &BtcDaiOrder> {
        self.inner
            .get(&self.me)
            .map(|orders| orders.values())
            .into_iter()
            .flatten()
    }

    /// Notify the OrderPool that we successfully setup a swap with a given
    /// quantity for one of our orders.
    ///
    /// While this was in progress, the OrderPool had "reserved" a certain
    /// quantity for this order. Now that we setup a swap successfully, we can
    /// clear this reservation and actually update the amount of the order.
    pub fn notify_swap_setup_successful(
        &mut self,
        order_id: OrderId,
        quantity: Quantity<asset::Bitcoin>,
    ) -> Result<()> {
        let quantity = quantity.to_inner();

        if let Some(reserved_quantity) = self.reserved_quantities.get_mut(&order_id) {
            if *reserved_quantity < quantity {
                anyhow::bail!(
                    "attempted to un-reserve {} but only {} were reserved",
                    reserved_quantity,
                    *reserved_quantity
                );
            }

            *reserved_quantity -= quantity;
        } else {
            tracing::warn!("we never reserved anything for order {}", order_id);
        }

        if let Some(our_orders) = self.inner.get_mut(&self.me) {
            if let Entry::Occupied(mut entry) = our_orders.entry(order_id) {
                let order = entry.get_mut();

                if order.quantity.to_inner() == quantity {
                    entry.remove();
                } else {
                    order.quantity = Quantity::new(order.quantity.to_inner() - quantity);
                }
            }
        }

        Ok(())
    }

    pub fn is_ours(&self, id: OrderId) -> bool {
        self.ours().any(|o| o.id == id)
    }

    pub fn matches(&mut self) -> Vec<Match> {
        let me = &self.me;

        let mut matches = Vec::new();

        // TODO: Figure out how to not duplicate this so the borrow-checker still gets
        // things
        let ours = self
            .inner
            .get(me)
            .map(|orders| orders.values())
            .into_iter()
            .flatten();
        let theirs = self
            .inner
            .iter()
            .filter_map(move |(maker, orders)| {
                if maker != me {
                    Some(iter::from_fn(move || Some(maker)).zip(orders.values()))
                } else {
                    None
                }
            })
            .flatten();

        for ours in ours {
            for (peer, theirs) in theirs.clone() {
                let reserved_ours = self
                    .reserved_quantities
                    .get(&ours.id)
                    .unwrap_or(&asset::Bitcoin::ZERO);
                let reserved_theirs = self
                    .reserved_quantities
                    .get(&theirs.id)
                    .unwrap_or(&asset::Bitcoin::ZERO);

                // TODO: Avoid the .clone() here somehow
                if self.no_match_cache.contains(&NoMatch {
                    ours: ours.clone(),
                    theirs: theirs.clone(),
                }) {
                    continue;
                }

                if let Some(r#match) = match_orders(ours, theirs, reserved_ours, reserved_theirs) {
                    let quantity = r#match.quantity;

                    matches.push(Match {
                        peer: peer.clone(),
                        price: r#match.price,
                        quantity,
                        ours: ours.id,
                        theirs: theirs.id,
                        our_position: ours.position,
                        swap_protocol: ours.swap_protocol,
                        match_reference_point: make_reference_point(ours, theirs),
                    });

                    // TODO: Expose these reservations to other parties
                    // Take care that we don't do it immediately, we probably want to wait until we
                    // successfully set up a swap
                    self.reserved_quantities
                        .entry(ours.id)
                        .or_default()
                        .add_assign(quantity.to_inner());

                    // TODO: We should reset this once we receive orders again from them
                    self.reserved_quantities
                        .entry(theirs.id)
                        .or_default()
                        .add_assign(quantity.to_inner());
                } else {
                    self.no_match_cache.put(
                        NoMatch {
                            ours: ours.clone(),
                            theirs: theirs.clone(),
                        },
                        (),
                    );
                }
            }
        }

        matches
    }
}

fn make_reference_point(left: &BtcDaiOrder, right: &BtcDaiOrder) -> OffsetDateTime {
    left.created_at.max(right.created_at)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub peer: PeerId,
    pub price: Price<asset::Bitcoin, Erc20Quantity>,
    pub quantity: Quantity<asset::Bitcoin>,
    pub ours: OrderId,
    pub theirs: OrderId,
    pub our_position: Position,
    pub swap_protocol: SwapProtocol,
    /// A reference point for this match.
    ///
    /// By definition, this is the more recent of the two creation timestamps of
    /// the orders this match is referencing.
    pub match_reference_point: OffsetDateTime,
}

impl Match {
    pub fn quote(&self) -> Erc20Quantity {
        self.quantity * self.price.clone()
    }
}

#[tracing::instrument(level = "debug", fields(left = %left.id, right = %right.id, %reserved_left, %reserved_right))]
fn match_orders(
    left: &BtcDaiOrder,
    right: &BtcDaiOrder,
    reserved_left: &asset::Bitcoin,
    reserved_right: &asset::Bitcoin,
) -> Option<InternalMatch> {
    use Position::*;

    let price = match (left.position, right.position) {
        (Sell, Buy) if left.price <= right.price => &left.price,
        (Buy, Sell) if left.price >= right.price => &right.price,
        (Sell, Sell) | (Buy, Buy) => {
            tracing::trace!("orders with the same position don't match");
            return None;
        }
        _ => {
            tracing::trace!(
                "{}ing at {} and {}ing at {} does not match",
                left.position,
                left.price.wei_per_sat(),
                right.position,
                right.price.wei_per_sat()
            );
            return None;
        }
    };

    if left.swap_protocol != right.swap_protocol {
        tracing::trace!("orders with different swap protocols don't match");
        return None;
    }

    // TODO: partial order matching
    if left.quantity != right.quantity {
        tracing::trace!("orders with different quantities don't match");
        return None;
    }

    let remaining_left = left.quantity.to_inner() - *reserved_left;
    let remaining_right = right.quantity.to_inner() - *reserved_right;

    if remaining_left == asset::Bitcoin::ZERO || remaining_right == asset::Bitcoin::ZERO {
        tracing::trace!("cannot fill order because of existing reserved funds");
        return None;
    }

    let quantity = remaining_left;

    tracing::info!("matched with {} at price {}", quantity, price.wei_per_sat());

    Some(InternalMatch {
        price: price.clone(),
        quantity: Quantity::new(quantity),
    })
}

// TODO: Find better name
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct InternalMatch {
    pub price: Price<asset::Bitcoin, Erc20Quantity>,
    pub quantity: Quantity<asset::Bitcoin>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset::Bitcoin,
        order::{btc, dai_per_btc},
        proptest,
    };
    use spectral::prelude::*;
    use time::NumericalDuration;

    #[test]
    fn given_two_orders_with_same_price_then_should_match() {
        let buy = BtcDaiOrder::buy(btc(1.0), dai_per_btc(9000), herc20_hbit());
        let sell = BtcDaiOrder::sell(btc(1.0), dai_per_btc(9000), herc20_hbit());

        let r#match = match_orders(&buy, &sell, &Bitcoin::ZERO, &Bitcoin::ZERO);

        assert_that(&r#match).is_some().is_equal_to(&InternalMatch {
            price: dai_per_btc(9000),
            quantity: btc(1.0),
        });
    }

    #[test]
    fn given_two_sell_orders_then_should_not_match() {
        let sell_1 = BtcDaiOrder::sell(btc(1.0), dai_per_btc(9000), herc20_hbit());
        let sell_2 = BtcDaiOrder::sell(btc(1.0), dai_per_btc(9000), herc20_hbit());

        let r#match = match_orders(&sell_1, &sell_2, &Bitcoin::ZERO, &Bitcoin::ZERO);

        assert_that(&r#match).is_none();
    }

    #[test]
    fn given_sell_for_9000_when_buy_for_8500_then_no_match() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai_per_btc(9000), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai_per_btc(8500), herc20_hbit());

        let r#match = match_orders(&sell, &buy, &Bitcoin::ZERO, &Bitcoin::ZERO);

        assert_that(&r#match).is_none();
    }

    #[test]
    fn given_sell_for_8500_when_buy_for_9000_then_match_at_8500() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai_per_btc(8500), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai_per_btc(9000), herc20_hbit());

        let r#match = match_orders(&sell, &buy, &Bitcoin::ZERO, &Bitcoin::ZERO);

        assert_that(&r#match).is_some().is_equal_to(&InternalMatch {
            price: dai_per_btc(8500),
            quantity: btc(1.0),
        });
    }

    // only temporary until we take care of partial matching properly
    #[test]
    fn given_different_quantities_then_no_match() {
        let sell = BtcDaiOrder::sell(btc(0.5), dai_per_btc(9000), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai_per_btc(9000), herc20_hbit());

        let r#match = match_orders(&sell, &buy, &Bitcoin::ZERO, &Bitcoin::ZERO);

        assert_that(&r#match).is_none();
    }

    #[test]
    fn given_reserved_quantity_then_only_matches_remaining_quantity() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai_per_btc(9000), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai_per_btc(9000), herc20_hbit());

        let r#match = match_orders(&sell, &buy, &btc(0.5).to_inner(), &Bitcoin::ZERO);

        assert_that(&r#match).is_some().is_equal_to(&InternalMatch {
            price: dai_per_btc(9000),
            quantity: btc(0.5),
        });
    }

    #[test]
    fn given_whole_order_reserved_then_no_match() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai_per_btc(9000), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai_per_btc(9000), herc20_hbit());

        let r#match = match_orders(&sell, &buy, &Bitcoin::ONE_BTC, &Bitcoin::ZERO);

        assert_that(&r#match).is_none();
    }

    #[test]
    fn given_different_swap_protocols_then_no_match() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai_per_btc(9000), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai_per_btc(9000), hbit_herc20());

        let r#match = match_orders(&sell, &buy, &Bitcoin::ZERO, &Bitcoin::ZERO);

        assert_that(&r#match).is_none();
    }

    #[test]
    fn given_same_swap_protocols_with_different_parameters_then_no_match() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai_per_btc(9000), SwapProtocol::HbitHerc20 {
            hbit_expiry_offset: 2.hours().into(),
            herc20_expiry_offset: 1.hours().into(),
        });
        let buy = BtcDaiOrder::buy(btc(1.0), dai_per_btc(9000), SwapProtocol::HbitHerc20 {
            hbit_expiry_offset: 3.hours().into(),
            herc20_expiry_offset: 1.hours().into(),
        });

        let r#match = match_orders(&sell, &buy, &Bitcoin::ZERO, &Bitcoin::ZERO);

        assert_that(&r#match).is_none();
    }

    #[test]
    fn make_reference_point_picks_the_more_recent_one() {
        let proto = BtcDaiOrder::buy(
            Quantity::new(Bitcoin::ZERO),
            Price::from_wei_per_sat(Erc20Quantity::zero()),
            hbit_herc20(),
        );

        let first = {
            let mut order = proto.clone();
            order.created_at = OffsetDateTime::from_unix_timestamp(0);
            order
        };
        let second = {
            let mut order = proto;
            order.created_at = OffsetDateTime::from_unix_timestamp(1000);
            order
        };

        let reference_point = make_reference_point(&first, &second);

        assert_eq!(reference_point, second.created_at);
    }

    proptest::proptest! {
        #[test]
        fn match_order_is_commutative(
            left in proptest::order::btc_dai_order(),
            right in proptest::order::btc_dai_order(),
            reserved_left in proptest::asset::bitcoin(),
            reserved_right in proptest::asset::bitcoin()
        ) {

            let first_match = match_orders(&left, &right, &reserved_left, &reserved_right);
            let second_match = match_orders(&right, &left, &reserved_right, &reserved_left);

            assert_eq!(first_match, second_match);
        }
    }

    #[test]
    fn orderpool_does_not_emit_the_same_match_twice() {
        let mut pool = OrderPool::new(PeerId::random());

        pool.publish(BtcDaiOrder::buy(btc(0.5), dai_per_btc(9000), hbit_herc20()));
        pool.receive(PeerId::random(), vec![BtcDaiOrder::sell(
            btc(0.5),
            dai_per_btc(9000),
            hbit_herc20(),
        )]);

        let matches_1 = pool.matches();
        let matches_2 = pool.matches();

        assert_that(&matches_1)
            .matching_contains(|m| m.price == dai_per_btc(9000) && m.quantity == btc(0.5));
        assert_that(&matches_2).has_length(0);
    }

    #[test]
    fn given_a_match_when_notified_about_successful_swap_then_removes_order_from_pool() {
        let mut pool = OrderPool::new(PeerId::random());

        let our_order = BtcDaiOrder::buy(btc(0.5), dai_per_btc(9000), hbit_herc20());
        pool.publish(our_order.clone());
        pool.receive(PeerId::random(), vec![BtcDaiOrder::sell(
            btc(0.5),
            dai_per_btc(9000),
            hbit_herc20(),
        )]);
        pool.matches();

        pool.notify_swap_setup_successful(our_order.id, btc(0.5))
            .unwrap();

        assert_that(&pool.ours().next()).is_none();
    }

    fn hbit_herc20() -> SwapProtocol {
        SwapProtocol::HbitHerc20 {
            hbit_expiry_offset: 0.seconds().into(),
            herc20_expiry_offset: 0.seconds().into(),
        }
    }

    fn herc20_hbit() -> SwapProtocol {
        SwapProtocol::Herc20Hbit {
            hbit_expiry_offset: 0.seconds().into(),
            herc20_expiry_offset: 0.seconds().into(),
        }
    }
}
