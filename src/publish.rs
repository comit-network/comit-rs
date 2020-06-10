struct Order {
    pub sell_amount: u32,
}

fn new_order(balance: u32) -> Order {
    Order {
        sell_amount: balance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_balance_return_order_selling_full_balance() {
        let balance = 10u32;

        let order = new_order(balance);

        assert_eq!(order.sell_amount, balance);
    }
}
