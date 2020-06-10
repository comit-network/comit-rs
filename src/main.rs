fn main() {
    println!("Hello, world!");
}

#[derive(Default)]
struct Order;

impl Order {
    pub fn new(peer: Peer) -> Order {
        Order
    }
}

#[derive(Copy, Clone, Default)]
struct Peer;

#[derive(Default)]
struct State;

impl State {
    fn proceed(&self, _order: Order) -> Result<(), ()> {
        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_taken_order_return_yes_proceed() {
        let state = State::default();

        let order = Order::default();

        let proceed = state.proceed(order);

        assert!(proceed.is_ok());
    }

    #[test]
    fn given_two_orders_from_same_peer_dont_proceed_second_one() {
        let state = State::default();

        let peer = Peer::default();

        let order_1 = Order::new(peer);
        let order_2 = Order::new(peer);

        let proceed_1 = state.proceed(order_1);

        let proceed_2 = state.proceed(order_2);

        assert!(proceed_1.is_ok());
        assert!(proceed_2.is_err());
    }
}
