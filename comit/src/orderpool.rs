use crate::{
    asset,
    asset::Erc20Quantity,
    order::{Denomination, SwapProtocol},
    BtcDaiOrder, OrderId, Position,
};
use libp2p::PeerId;
use std::{collections::HashMap, iter, iter::FromIterator, ops::AddAssign};
use time::OffsetDateTime;

/// A collection of orders gathered from several makers.
#[derive(Clone, Debug)]
pub struct OrderPool {
    inner: HashMap<PeerId, HashMap<OrderId, BtcDaiOrder>>,

    reserved_quantities: HashMap<OrderId, asset::Bitcoin>,
    /// Our own id.
    ///
    /// Allows us to filter out our own orders.
    me: PeerId,
}

impl OrderPool {
    pub fn new(me: PeerId) -> Self {
        Self {
            inner: Default::default(),
            reserved_quantities: Default::default(),
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
    /// This replaces all current orders this peer with the newly received ones.
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
                        .add_assign(quantity);

                    // TODO: We should reset this once we receive orders again from them
                    self.reserved_quantities
                        .entry(theirs.id)
                        .or_default()
                        .add_assign(quantity);
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
    pub price: Erc20Quantity,
    pub quantity: asset::Bitcoin,
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

#[tracing::instrument(level = "debug", fields(left = %left.id, right = %right.id, %reserved_left, %reserved_right))]
fn match_orders(
    left: &BtcDaiOrder,
    right: &BtcDaiOrder,
    reserved_left: &asset::Bitcoin,
    reserved_right: &asset::Bitcoin,
) -> Option<InternalMatch> {
    use Denomination::WeiPerSat;
    use Position::*;

    let price_left = left.price(WeiPerSat);
    let price_right = right.price(WeiPerSat);

    let price = match (left.position, right.position) {
        (Sell, Buy) if price_left <= price_right => price_left,
        (Buy, Sell) if price_left >= price_right => price_right,
        (Sell, Sell) | (Buy, Buy) => {
            tracing::trace!("orders with the same position don't match");
            return None;
        }
        _ => {
            tracing::trace!(
                "{}ing at {} and {}ing at {} does not match",
                left.position,
                price_left,
                right.position,
                price_right
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

    let remaining_left = left.quantity - *reserved_left;
    let remaining_right = right.quantity - *reserved_right;

    if remaining_left == asset::Bitcoin::ZERO || remaining_right == asset::Bitcoin::ZERO {
        tracing::trace!("cannot fill order because of existing reserved funds");
        return None;
    }

    let quantity = remaining_left;

    tracing::trace!("matched with {} at price {}", quantity, price);

    Some(InternalMatch { price, quantity })
}

// TODO: Find better name
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct InternalMatch {
    pub price: Erc20Quantity,
    pub quantity: asset::Bitcoin,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset::{bitcoin::btc, ethereum::dai},
        proptest,
    };
    use spectral::prelude::*;
    use time::NumericalDuration;

    #[test]
    fn given_two_orders_with_same_price_then_should_match() {
        let buy = BtcDaiOrder::buy(btc(1.0), dai(9000), herc20_hbit());
        let sell = BtcDaiOrder::sell(btc(1.0), dai(9000), herc20_hbit());

        let r#match = match_orders(&buy, &sell, &btc(0.0), &btc(0.0));

        assert_that(&r#match).is_some().is_equal_to(&InternalMatch {
            price: dai(9000),
            quantity: btc(1.0),
        });
    }

    #[test]
    fn given_two_sell_orders_then_should_not_match() {
        let sell_1 = BtcDaiOrder::sell(btc(1.0), dai(9000), herc20_hbit());
        let sell_2 = BtcDaiOrder::sell(btc(1.0), dai(9000), herc20_hbit());

        let r#match = match_orders(&sell_1, &sell_2, &btc(0.0), &btc(0.0));

        assert_that(&r#match).is_none();
    }

    #[test]
    fn given_sell_for_9000_when_buy_for_8500_then_no_match() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai(9000), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai(8500), herc20_hbit());

        let r#match = match_orders(&sell, &buy, &btc(0.0), &btc(0.0));

        assert_that(&r#match).is_none();
    }

    #[test]
    fn given_sell_for_8500_when_buy_for_9000_then_match_at_8500() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai(8500), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai(9000), herc20_hbit());

        let r#match = match_orders(&sell, &buy, &btc(0.0), &btc(0.0));

        assert_that(&r#match).is_some().is_equal_to(&InternalMatch {
            price: dai(8500),
            quantity: btc(1.0),
        });
    }

    // only temporary until we take care of partial matching properly
    #[test]
    fn given_different_quantities_then_no_match() {
        let sell = BtcDaiOrder::sell(btc(0.5), dai(9000), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai(9000), herc20_hbit());

        let r#match = match_orders(&sell, &buy, &btc(0.0), &btc(0.0));

        assert_that(&r#match).is_none();
    }

    #[test]
    fn given_reserved_quantity_then_only_matches_remaining_quantity() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai(9000), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai(9000), herc20_hbit());

        let r#match = match_orders(&sell, &buy, &btc(0.5), &btc(0.0));

        assert_that(&r#match).is_some().is_equal_to(&InternalMatch {
            price: dai(9000),
            quantity: btc(0.5),
        });
    }

    #[test]
    fn given_whole_order_reserved_then_no_match() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai(9000), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai(9000), herc20_hbit());

        let r#match = match_orders(&sell, &buy, &btc(1.0), &btc(0.0));

        assert_that(&r#match).is_none();
    }

    #[test]
    fn given_different_swap_protocols_then_no_match() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai(9000), herc20_hbit());
        let buy = BtcDaiOrder::buy(btc(1.0), dai(9000), hbit_herc20());

        let r#match = match_orders(&sell, &buy, &btc(0.0), &btc(0.0));

        assert_that(&r#match).is_none();
    }

    #[test]
    fn given_same_swap_protocols_with_different_parameters_then_no_match() {
        let sell = BtcDaiOrder::sell(btc(1.0), dai(9000), SwapProtocol::HbitHerc20 {
            hbit_expiry_offset: 2.hours(),
            herc20_expiry_offset: 1.hours(),
        });
        let buy = BtcDaiOrder::buy(btc(1.0), dai(9000), SwapProtocol::HbitHerc20 {
            hbit_expiry_offset: 3.hours(),
            herc20_expiry_offset: 1.hours(),
        });

        let r#match = match_orders(&sell, &buy, &btc(0.0), &btc(0.0));

        assert_that(&r#match).is_none();
    }

    #[test]
    fn make_reference_point_picks_the_more_recent_one() {
        let proto = BtcDaiOrder::buy(Default::default(), Erc20Quantity::zero(), hbit_herc20());

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

        pool.publish(BtcDaiOrder::buy(btc(0.5), dai(9000), hbit_herc20()));
        pool.receive(PeerId::random(), vec![BtcDaiOrder::sell(
            btc(0.5),
            dai(9000),
            hbit_herc20(),
        )]);

        let matches_1 = pool.matches();
        let matches_2 = pool.matches();

        assert_that(&matches_1)
            .matching_contains(|m| m.price == dai(9000) && m.quantity == btc(0.5));
        assert_that(&matches_2).has_length(0);
    }

    fn hbit_herc20() -> SwapProtocol {
        SwapProtocol::HbitHerc20 {
            hbit_expiry_offset: Default::default(),
            herc20_expiry_offset: Default::default(),
        }
    }

    fn herc20_hbit() -> SwapProtocol {
        SwapProtocol::Herc20Hbit {
            hbit_expiry_offset: Default::default(),
            herc20_expiry_offset: Default::default(),
        }
    }
}
