fn main() {
    println!("Hello, world!");
}

#[derive(Default)]
struct Order;

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
}
