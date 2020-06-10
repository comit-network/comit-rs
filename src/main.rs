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

fn should_proceed(_order: Order) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_taken_order_return_yes_proceed() {
        let order = Order::default();

        let proceed = should_proceed(order);

        assert!(proceed);
    }

    #[test]
    fn given_two_orders_from_same_peer_dont_proceed_second_one() {
        let peer = Peer::default();

        let order_1 = Order::new(peer);
        let order_2 = Order::new(peer);

        let proceed_1 = should_proceed(order_1);

        let proceed_2 = should_proceed(order_2);

        assert!(proceed_1);
        assert!(!proceed_2);
    }
}
