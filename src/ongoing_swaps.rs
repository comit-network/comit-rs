use std::collections::HashSet;

#[derive(Copy, Clone)]
struct Order {
    pub peer: Peer,
}

impl Order {
    pub fn new(peer: Peer) -> Order {
        Order { peer }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct Peer(u32);

impl Peer {
    fn new(index: u32) -> Peer {
        Peer(index)
    }
}

#[derive(Default)]
struct OngoingSwaps {
    peers: HashSet<Peer>,
}

impl OngoingSwaps {
    fn insert(&mut self, order: Order) -> Result<(), ()> {
        if self.peers.contains(&order.peer) {
            Err(())
        } else {
            self.peers.insert(order.peer);
            Ok(())
        }
    }

    fn remove(&mut self, order: &Order) {
        self.peers.remove(&order.peer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_taken_order_return_yes_proceed() {
        let mut ongoing_swaps = OngoingSwaps::default();

        let order = Order::new(Peer::new(0));

        let insertion = ongoing_swaps.insert(order);

        assert!(insertion.is_ok());
    }

    #[test]
    fn given_two_orders_from_same_peer_dont_proceed_second_one() {
        let mut state = OngoingSwaps::default();

        let peer = Peer::new(0);

        let order_1 = Order::new(peer);
        let order_2 = Order::new(peer);

        let insertion_1 = state.insert(order_1);

        let insertion_2 = state.insert(order_2);

        assert!(insertion_1.is_ok());
        assert!(insertion_2.is_err());
    }

    #[test]
    fn given_two_orders_from_diff_peer_do_proceed_with_both() {
        let mut state = OngoingSwaps::default();

        let peer_1 = Peer::new(1);
        let peer_2 = Peer::new(2);

        let order_1 = Order::new(peer_1);
        let order_2 = Order::new(peer_2);

        let insertion_1 = state.insert(order_1);

        let insertion_2 = state.insert(order_2);

        assert!(insertion_1.is_ok());
        assert!(insertion_2.is_ok());
    }

    #[test]
    fn given_two_orders_from_same_peer_do_proceed_if_first_execution_is_done() {
        let mut state = OngoingSwaps::default();

        let peer = Peer::new(0);

        let order_1 = Order::new(peer);
        let order_2 = Order::new(peer);

        let insertion_1 = state.insert(order_1);
        // Execution is not represented in the test, order should be removed once execution is done.
        state.remove(&order_1);

        let insertion_2 = state.insert(order_2);

        assert!(insertion_1.is_ok());
        assert!(insertion_2.is_ok());
    }
}
