use std::cmp::min;

pub trait LockedFunds {
    fn locked_funds(&self) -> u32;
}

pub trait Balance {
    fn balance(&self) -> u32;
}

pub trait Fees {
    fn fees(&self) -> u32;
}

struct Order {
    pub sell_amount: u32,
}

fn new_order<W, B>(wallet: W, book: B, max_amount: u32) -> Order
where
    W: Balance + Fees,
    B: LockedFunds,
{
    Order {
        sell_amount: min(wallet.balance() - book.locked_funds(), max_amount) - wallet.fees(),
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
        fees: u32,
    }

    impl Wallet {
        fn new(balance: u32, fees: u32) -> Wallet {
            Wallet { balance, fees }
        }
    }

    impl Balance for Wallet {
        fn balance(&self) -> u32 {
            self.balance
        }
    }

    impl Fees for Wallet {
        fn fees(&self) -> u32 {
            self.fees
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
        let wallet = Wallet::new(10, 0);

        let book = Book::new(0);

        let order = new_order(wallet, book, 100);

        assert_eq!(order.sell_amount, 10u32);
    }

    #[test]
    fn given_a_balance_and_locked_funds_return_order_selling_available_balance() {
        let wallet = Wallet::new(10, 0);

        let book = Book::new(2);

        let order = new_order(wallet, book, 100);

        assert_eq!(order.sell_amount, 8u32);
    }

    #[test]
    fn given_an_available_balance_and_a_max_amount_sell_min_of_either() {
        let wallet = Wallet::new(10, 0);

        let book = Book::new(2);

        let order = new_order(wallet, book, 2);

        assert_eq!(order.sell_amount, 2u32);
    }

    #[test]
    fn given_an_available_balance_and_fees_sell_balance_minus_fees() {
        let wallet = Wallet::new(10, 1);

        let book = Book::new(2);

        let order = new_order(wallet, book, 2);

        assert_eq!(order.sell_amount, 1u32);
    }
}
