pub trait LockedFunds {
    fn locked_funds(&self) -> u32;
}

pub trait Balance {
    fn balance(&self) -> u32;
}

struct Order {
    pub sell_amount: u32,
}

fn new_order<W, B>(wallet: W, book: B) -> Order
where
    W: Balance,
    B: LockedFunds,
{
    Order {
        sell_amount: wallet.balance() - book.locked_funds(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Book {
        locked_funds: u32,
    }

    struct Wallet {
        balance: u32,
    }

    impl Wallet {
        fn new(balance: u32) -> Wallet {
            Wallet { balance }
        }
    }

    impl Balance for Wallet {
        fn balance(&self) -> u32 {
            self.balance
        }
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
        let wallet = Wallet::new(10);

        let book = Book::new(0);

        let order = new_order(wallet, book);

        assert_eq!(order.sell_amount, 10u32);
    }

    #[test]
    fn given_a_balance_and_locked_funds_return_order_selling_available_balance() {
        let wallet = Wallet::new(10);

        let book = Book::new(2);

        let order = new_order(wallet, book);

        assert_eq!(order.sell_amount, 8u32);
    }
}
