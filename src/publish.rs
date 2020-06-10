pub trait LockedFunds {
    fn locked_funds(&self) -> u32;
}

struct Order {
    pub sell_amount: u32,
}

fn new_order<B>(balance: u32, book: B) -> Order
where
    B: LockedFunds,
{
    Order {
        sell_amount: balance - book.locked_funds(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Book {
        locked_funds: u32,
    }

    impl Book {
        fn new(locked_funds: u32) -> Book {
            Book { locked_funds }
        }
    }

    impl LockedFunds for Book {
        fn locked_funds(&self) -> u32 {
            self.locked_funds
        }
    }

    #[test]
    fn given_a_balance_return_order_selling_full_balance() {
        let balance = 10u32;

        let book = Book::new(0);

        let order = new_order(balance, book);

        assert_eq!(order.sell_amount, 10u32);
    }

    #[test]
    fn given_a_balance_and_locked_funds_return_order_selling_available_balance() {
        let balance = 10u32;

        let book = Book::new(2);

        let order = new_order(balance, book);

        assert_eq!(order.sell_amount, 8u32);
    }
}
