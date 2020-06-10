struct Order {
    pub sell_amount: u32,
}

fn new_order(balance: u32, locked_funds: u32) -> Order {
    Order {
        sell_amount: balance - locked_funds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_balance_return_order_selling_full_balance() {
        let balance = 10u32;

        let order = new_order(balance, 0);

        assert_eq!(order.sell_amount, 10u32);
    }

    #[test]
    fn given_a_balance_and_locked_funds_return_order_selling_available_balance() {
        let balance = 10u32;

        let locked_funds = 2u32;

        let order = new_order(balance, locked_funds);

        assert_eq!(order.sell_amount, 8u32);
    }
}
