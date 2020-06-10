fn main() {
    println!("Hello, world!");
}

#[derive(Copy, Clone)]
struct Order {
    pub peer: Peer
}

impl Order {
    pub fn new(peer: Peer) -> Order {
        Order { peer }
    }
}

#[derive(Copy, Clone, Default, Eq, PartialEq)]
struct Peer;

#[derive(Default)]
struct State {
    peer_with_ongoing_orders: Vec<Peer>
}

impl State {
    fn proceed(&mut self, order: Order) -> Result<(), ()> {
        if self.peer_with_ongoing_orders.contains(&order.peer) {
            Err(())
        } else {
            self.peer_with_ongoing_orders.push(order.peer);
            Ok(())
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_taken_order_return_yes_proceed() {
        let mut state = State::default();

        let order = Order::new(Peer::default());

        let proceed = state.proceed(order);

        assert!(proceed.is_ok());
    }

    #[test]
    fn given_two_orders_from_same_peer_dont_proceed_second_one() {
        let mut state = State::default();

        let peer = Peer::default();

        let order_1 = Order::new(peer);
        let order_2 = Order::new(peer);

        let proceed_1 = state.proceed(order_1);

        let proceed_2 = state.proceed(order_2);

        assert!(proceed_1.is_ok());
        assert!(proceed_2.is_err());
    }
}
