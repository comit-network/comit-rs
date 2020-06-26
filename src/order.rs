//! The maker creates an order that defines how much he wants to buy for the amount he is selling.
//! order's buy amount = what the maker wants from a taker
//! order's sell amount = what the maker is offering to a taker
//!
//! mid_market_rate is set as 1 sell => x buy, where x is the mid_market_rate
//!
//! BTC-DAI: When selling 1 BTC we should buy 9000 DAI, mid_market_rate is 1:9000
//! Given BTC:DAI and the rate of 1:9000
//!     selling 1.0 BTC with spread_pc of 3% => buy 9270 DAI
//!     selling 0.5 BTC with spread_pc of 3% => buy 4635 DAI
//! Given DAI:BTC and a rate of 1:0.0001
//!     selling 10000 DAI with spread_pc of 3% => buy 1.03 BTC
//!     selling 1000 DAI with spread_pc of 3% => buy 0.103 DAI
//!

use crate::bitcoin;
use crate::dai;
use crate::{Rate, Spread};
use std::cmp::min;

#[derive(Debug, Clone)]
pub struct BtcDaiOrder<P> {
    pub position: P,
    pub base: bitcoin::Amount,
    pub quote: dai::Amount,
}

#[derive(Debug, Clone, Copy)]
pub struct Buy;

#[derive(Debug, Clone, Copy)]
pub struct Sell;

impl BtcDaiOrder<Buy> {
    pub fn new<W, B>(
        _wallet: W,
        _book: B,
        _max_sell_amount: dai::Amount,
        _mid_market_rate: Rate,
        _spread: Spread,
    ) -> anyhow::Result<BtcDaiOrder<Buy>>
    where
        W: Balance<Amount = dai::Amount> + Fees<Amount = dai::Amount>,
        B: LockedFunds<Amount = dai::Amount>,
    {
        todo!();
    }
}

impl BtcDaiOrder<Sell> {
    pub fn new<W, B>(
        wallet: W,
        book: B,
        max_sell_amount: bitcoin::Amount,
        mid_market_rate: Rate,
        spread: Spread,
    ) -> anyhow::Result<BtcDaiOrder<Sell>>
    where
        W: Balance<Amount = bitcoin::Amount> + Fees<Amount = bitcoin::Amount>,
        B: LockedFunds<Amount = bitcoin::Amount>,
    {
        let base = min(wallet.balance() - book.locked_funds(), max_sell_amount) - wallet.fees();

        let rate = spread.apply(mid_market_rate)?;
        let quote = base.worth_in(rate);

        Ok(BtcDaiOrder {
            position: Sell,
            base,
            quote,
        })
    }
}

pub trait LockedFunds {
    type Amount;
    fn locked_funds(&self) -> Self::Amount;
}

pub trait Balance {
    type Amount;
    fn balance(&self) -> Self::Amount;
}

pub trait Fees {
    type Amount;
    fn fees(&self) -> Self::Amount;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rate;
    use std::convert::TryFrom;

    #[derive(Copy, Clone)]
    struct Book {
        locked_funds: bitcoin::Amount,
    }

    #[derive(Copy, Clone)]
    struct Wallet {
        balance: bitcoin::Amount,
        fees: bitcoin::Amount,
    }

    impl Wallet {
        fn new<A: Into<bitcoin::Amount>>(balance: A, fees: A) -> Wallet {
            Wallet {
                balance: balance.into(),
                fees: fees.into(),
            }
        }
    }

    impl Balance for Wallet {
        type Amount = bitcoin::Amount;
        fn balance(&self) -> Self::Amount {
            self.balance
        }
    }

    impl Fees for Wallet {
        type Amount = bitcoin::Amount;
        fn fees(&self) -> Self::Amount {
            self.fees
        }
    }

    impl Book {
        fn new<A: Into<bitcoin::Amount>>(locked_funds: A) -> Book {
            Book {
                locked_funds: locked_funds.into(),
            }
        }
    }

    impl LockedFunds for Book {
        type Amount = bitcoin::Amount;
        fn locked_funds(&self) -> Self::Amount {
            self.locked_funds
        }
    }

    fn btc(btc: f64) -> bitcoin::Amount {
        bitcoin::Amount::from_btc(btc).unwrap()
    }

    fn dai(dai: f64) -> dai::Amount {
        dai::Amount::from_dai_trunc(dai).unwrap()
    }

    #[test]
    fn given_a_balance_return_order_selling_full_balance() {
        let wallet = Wallet::new(btc(10.0), btc(0.0));
        let book = Book::new(btc(0.0));

        let rate = Rate::try_from(1.0).unwrap();
        let order =
            BtcDaiOrder::<Sell>::new(wallet, book, btc(100.0), rate, Spread::new(0).unwrap())
                .unwrap();

        assert_eq!(order.base, btc(10.0));
    }

    #[test]
    fn given_a_balance_and_locked_funds_return_order_selling_available_balance() {
        let wallet = Wallet::new(btc(10.0), btc(0.0));
        let book = Book::new(btc(2.0));

        let rate = Rate::try_from(1.0).unwrap();
        let order =
            BtcDaiOrder::<Sell>::new(wallet, book, btc(100.0), rate, Spread::new(0).unwrap())
                .unwrap();

        assert_eq!(order.base, btc(8.0));
    }

    #[test]
    fn given_an_available_balance_and_a_max_amount_sell_min_of_either() {
        let wallet = Wallet::new(btc(10.0), btc(0.0));
        let book = Book::new(btc(2.0));

        let rate = Rate::try_from(1.0).unwrap();
        let order = BtcDaiOrder::<Sell>::new(wallet, book, btc(2.0), rate, Spread::new(0).unwrap())
            .unwrap();

        assert_eq!(order.base, btc(2.0));
    }

    #[test]
    fn given_an_available_balance_and_fees_sell_balance_minus_fees() {
        let wallet = Wallet::new(btc(10.0), btc(1.0));
        let book = Book::new(btc(2.0));

        let rate = Rate::try_from(1.0).unwrap();
        let order = BtcDaiOrder::<Sell>::new(wallet, book, btc(2.0), rate, Spread::new(0).unwrap())
            .unwrap();

        assert_eq!(order.base, btc(1.0));
    }

    #[test]
    fn given_a_rate_return_order_with_both_amounts() {
        let wallet = Wallet::new(btc(1051.0), btc(1.0));
        let book = Book::new(btc(50.0));
        let spread = Spread::new(0).unwrap();

        let rate = Rate::try_from(0.1).unwrap();

        let order = BtcDaiOrder::<Sell>::new(wallet, book, btc(9999.0), rate, spread).unwrap();

        // 1 Sell => 0.1 Buy
        // 1000 Sell => 100 Buy
        assert_eq!(order.base, btc(1000.0));
        assert_eq!(order.quote, dai(100.0));

        let rate = Rate::try_from(10.0).unwrap();

        let order = BtcDaiOrder::<Sell>::new(wallet, book, btc(9999.0), rate, spread).unwrap();

        assert_eq!(order.base, btc(1000.0));
        assert_eq!(order.quote, dai(10_000.0));
    }

    #[test]
    fn given_a_rate_and_spread_return_order_with_both_amounts() {
        let wallet = Wallet::new(btc(1051.0), btc(1.0));
        let book = Book::new(btc(50.0));
        let rate = Rate::try_from(0.1).unwrap();
        let spread = Spread::new(300).unwrap();

        let order = BtcDaiOrder::<Sell>::new(wallet, book, btc(9999.0), rate, spread).unwrap();

        assert_eq!(order.base, btc(1000.0));
        assert_eq!(order.quote, dai(103.0));
    }
}
