use std::cmp::min;

pub trait LockedFunds {
    fn locked_funds(&self) -> u64;
}

pub trait Balance {
    fn balance(&self) -> u64;
}

pub trait Fees {
    fn fees(&self) -> u64;
}

struct Order {
    pub sell_amount: u64,
    pub buy_amount: u64,
}

/// Contains a positive percentage value expressed in ratio: 1 is 100%
/// To avoid human errors, the max value is 1.
struct Spread(f64);

impl Spread {
    pub fn new(spread: f64) -> Result<Spread, ()> {
        if spread.is_sign_positive() && spread <= 1.0 {
            Ok(Spread(spread))
        } else {
            Err(())
        }
    }

    pub fn as_f64(&self) -> f64 {
        self.0
    }
}

/// mid_market_rate is buy/sell: 1 Buy => mid_market_rate Sell: = sell/buy
/// spread_pc: percent value to be added to the buy amount
#[allow(clippy::cast_precision_loss)] // It's ok because it just means we are applying slightly more than the given spread
fn new_order<W, B>(
    sell_wallet: W,
    book: B,
    max_sell_amount: u64,
    mid_market_rate: f64,
    spread: Spread,
) -> Order
where
    W: Balance + Fees,
    B: LockedFunds,
{
    let sell_amount =
        min(sell_wallet.balance() - book.locked_funds(), max_sell_amount) - sell_wallet.fees();

    let rate = mid_market_rate / (1.0 + spread.as_f64());

    let buy_amount = sell_amount as f64 / rate;

    Order {
        sell_amount,
        buy_amount: buy_amount.ceil() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Book {
        locked_funds: u64,
    }

    struct Wallet {
        balance: u64,
        fees: u64,
    }

    impl Wallet {
        fn new(balance: u64, fees: u64) -> Wallet {
            Wallet { balance, fees }
        }
    }

    impl Balance for Wallet {
        fn balance(&self) -> u64 {
            self.balance
        }
    }

    impl Fees for Wallet {
        fn fees(&self) -> u64 {
            self.fees
        }
    }

    impl Book {
        fn new(locked_funds: u64) -> Book {
            Book { locked_funds }
        }
    }

    impl LockedFunds for Book {
        fn locked_funds(&self) -> u64 {
            self.locked_funds
        }
    }

    #[test]
    fn given_a_balance_return_order_selling_full_balance() {
        let wallet = Wallet::new(10, 0);

        let book = Book::new(0);

        let order = new_order(wallet, book, 100, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.sell_amount, 10);
    }

    #[test]
    fn given_a_balance_and_locked_funds_return_order_selling_available_balance() {
        let wallet = Wallet::new(10, 0);

        let book = Book::new(2);

        let order = new_order(wallet, book, 100, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.sell_amount, 8);
    }

    #[test]
    fn given_an_available_balance_and_a_max_amount_sell_min_of_either() {
        let wallet = Wallet::new(10, 0);

        let book = Book::new(2);

        let order = new_order(wallet, book, 2, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.sell_amount, 2);
    }

    #[test]
    fn given_an_available_balance_and_fees_sell_balance_minus_fees() {
        let wallet = Wallet::new(10, 1);

        let book = Book::new(2);

        let order = new_order(wallet, book, 2, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.sell_amount, 1);
    }

    #[test]
    fn given_a_rate_return_order_with_both_amounts() {
        let wallet = Wallet::new(1051, 1);

        let book = Book::new(50);

        let order = new_order(wallet, book, 9999, 10.0, Spread::new(0.0).unwrap());
        // 1 Buy => 10 Sell
        // ? Buy => 1000 sell
        // 100 Buy => 1000 Sell

        assert_eq!(order.sell_amount, 1000);
        assert_eq!(order.buy_amount, 100)
    }

    #[test]
    fn given_a_rate_and_spread_return_order_with_both_amounts() {
        let wallet = Wallet::new(1051, 1);

        let book = Book::new(50);

        let order = new_order(wallet, book, 9999, 10.0, Spread::new(0.03).unwrap());
        // 1 Buy => 10 Sell
        // ? Buy => 1000 sell
        // 100 Buy => 1000 Sell
        // 3% spread
        // 103 Buy => 1000 Sell

        assert_eq!(order.sell_amount, 1000);
        assert_eq!(order.buy_amount, 104); // Rounding up taking in account precision loss
    }
}
