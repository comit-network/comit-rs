use crate::{
    bitcoin,
    ethereum::{
        self,
        dai::{self, DaiContractAddress},
    },
    MidMarketRate, Rate, Spread,
};
use std::{cmp::min, convert::TryFrom};

#[derive(Debug, Copy, Clone, strum_macros::Display)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Symbol {
    Btc,
    Dai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::Display)]
pub enum Position {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtcDaiOrder {
    pub position: Position,
    pub base: bitcoin::Amount,
    pub quote: dai::Asset,
}

impl BtcDaiOrder {
    #[allow(clippy::too_many_arguments)]
    pub fn new_sell(
        base_balance: bitcoin::Amount,
        base_fees: bitcoin::Amount,
        base_reserved_funds: bitcoin::Amount,
        max_amount: Option<bitcoin::Amount>,
        mid_market_rate: Rate,
        spread: Spread,
        dai_contract_address: DaiContractAddress,
        chain_id: ethereum::ChainId,
    ) -> anyhow::Result<BtcDaiOrder> {
        if let Some(max_amount) = max_amount {
            if max_amount < base_fees {
                anyhow::bail!(MaxAmountSmallerThanMaxFee)
            }
        }

        match base_reserved_funds.checked_add(base_fees) {
            Some(added) => {
                if base_balance <= added {
                    anyhow::bail!(InsufficientFunds(Symbol::Btc))
                }
            }
            None => anyhow::bail!(Overflow),
        }

        let base = match max_amount {
            Some(max_amount) => min(base_balance - base_reserved_funds, max_amount) - base_fees,
            None => base_balance - base_reserved_funds - base_fees,
        };

        let rate = spread.apply(mid_market_rate, Position::Sell)?;
        let quote_amount = base.worth_in(rate);

        let quote = dai::Asset {
            amount: quote_amount,
            contract_address: dai_contract_address,
            chain_id,
        };

        Ok(BtcDaiOrder {
            position: Position::Sell,
            base,
            quote,
        })
    }

    pub fn new_buy(
        quote_balance: dai::Amount,
        quote_reserved_funds: dai::Amount,
        max_amount: Option<dai::Amount>,
        mid_market_rate: Rate,
        spread: Spread,
        dai_contract_address: DaiContractAddress,
        chain_id: ethereum::ChainId,
    ) -> anyhow::Result<BtcDaiOrder> {
        if quote_balance <= quote_reserved_funds {
            anyhow::bail!(InsufficientFunds(Symbol::Dai))
        }

        let quote_amount = match max_amount {
            Some(max_amount) => min(quote_balance - quote_reserved_funds, max_amount),
            None => quote_balance - quote_reserved_funds,
        };

        let rate = spread.apply(mid_market_rate, Position::Buy)?;
        let base = quote_amount.worth_in(rate)?;

        let quote = dai::Asset {
            amount: quote_amount,
            contract_address: dai_contract_address,
            chain_id,
        };

        Ok(BtcDaiOrder {
            position: Position::Buy,
            base,
            quote,
        })
    }

    pub fn is_as_good_as(&self, mid_market_rate: MidMarketRate) -> anyhow::Result<bool> {
        let order_rate = Rate::try_from(self.clone())?;
        match self.position {
            Position::Buy => {
                // We are buying BTC for DAI
                // Given an order rate of: 1:9000
                // It is NOT profitable to buy, if the current rate is greater than the order rate.
                // 1:8800 -> We give less DAI for getting BTC -> Good.
                // 1:9200 -> We have to give more DAI for getting BTC -> Sucks.
                Ok(order_rate <= mid_market_rate.into())
            }
            Position::Sell => {
                // We are selling BTC for DAI
                // Given an order rate of: 1:9000
                // It is NOT profitable to sell, if the current rate is smaller than the order rate.
                // 1:8800 -> We get less DAI for our BTC -> Sucks.
                // 1:9200 -> We get more DAI for our BTC -> Good.
                Ok(order_rate >= mid_market_rate.into())
            }
        }
    }
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("Insufficient {0} funds to create new order.")]
pub struct InsufficientFunds(Symbol);

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("The maximum amount for an order cannot be smaller than the maximum fee.")]
pub struct MaxAmountSmallerThanMaxFee;

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("Amounts to large to be added.")]
pub struct Overflow;

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

impl From<Position> for comit::network::Position {
    fn from(from: Position) -> Self {
        match from {
            Position::Buy => comit::network::Position::Buy,
            Position::Sell => comit::network::Position::Sell,
        }
    }
}

impl From<comit::network::Position> for Position {
    fn from(from: comit::network::Position) -> Self {
        match from {
            comit::network::Position::Buy => Position::Buy,
            comit::network::Position::Sell => Position::Sell,
        }
    }
}

#[cfg(test)]
impl Default for BtcDaiOrder {
    fn default() -> Self {
        Self {
            position: Position::Buy,
            base: bitcoin::Amount::from_sat(1),
            quote: dai::Asset {
                amount: dai::Amount::from_atto(num::BigUint::from(1u8)),
                contract_address: DaiContractAddress::Mainnet,
                chain_id: ethereum::ChainId::mainnet(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rate;
    use num::BigUint;
    use proptest::prelude::*;
    use std::convert::TryFrom;
    use std::str::FromStr;

    fn btc(btc: f64) -> bitcoin::Amount {
        bitcoin::Amount::from_btc(btc).unwrap()
    }

    fn dai_amount(dai: f64) -> dai::Amount {
        dai::Amount::from_dai_trunc(dai).unwrap()
    }

    fn dai_asset(dai: f64) -> dai::Asset {
        dai::Asset {
            amount: dai_amount(dai),
            contract_address: DaiContractAddress::Mainnet,
            chain_id: ethereum::ChainId::mainnet(),
        }
    }

    #[test]
    fn given_a_balance_return_order_selling_full_balance() {
        let rate = Rate::try_from(1.0).unwrap();
        let order = BtcDaiOrder::new_sell(
            btc(10.0),
            btc(0.0),
            btc(0.0),
            Some(btc(100.0)),
            rate,
            Spread::new(0).unwrap(),
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        assert_eq!(order.base, btc(10.0));

        let order = BtcDaiOrder::new_buy(
            dai_amount(10.0),
            dai_amount(0.0),
            Some(dai_amount(100.0)),
            rate,
            Spread::new(0).unwrap(),
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        assert_eq!(order.quote, dai_asset(10.0));
    }

    #[test]
    fn given_a_balance_and_locked_funds_return_order_selling_available_balance() {
        let rate = Rate::try_from(1.0).unwrap();
        let order = BtcDaiOrder::new_sell(
            btc(10.0),
            btc(0.0),
            btc(2.0),
            Some(btc(100.0)),
            rate,
            Spread::new(0).unwrap(),
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        assert_eq!(order.base, btc(8.0));

        let order = BtcDaiOrder::new_buy(
            dai_amount(10.0),
            dai_amount(2.0),
            None,
            rate,
            Spread::new(0).unwrap(),
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        assert_eq!(order.quote, dai_asset(8.0));
    }

    #[test]
    fn given_an_available_balance_and_a_max_amount_sell_min_of_either() {
        let rate = Rate::try_from(1.0).unwrap();
        let order = BtcDaiOrder::new_sell(
            btc(10.0),
            btc(0.0),
            btc(2.0),
            Some(btc(2.0)),
            rate,
            Spread::new(0).unwrap(),
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        assert_eq!(order.base, btc(2.0));

        let order = BtcDaiOrder::new_buy(
            dai_amount(10.0),
            dai_amount(2.0),
            Some(dai_amount(2.0)),
            rate,
            Spread::new(0).unwrap(),
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        assert_eq!(order.quote, dai_asset(2.0));
    }

    #[test]
    fn given_an_available_balance_and_fees_sell_balance_minus_fees() {
        let rate = Rate::try_from(1.0).unwrap();
        let order = BtcDaiOrder::new_buy(
            dai_amount(10.0),
            dai_amount(3.0),
            Some(dai_amount(1.0)),
            rate,
            Spread::new(0).unwrap(),
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        assert_eq!(order.quote, dai_asset(1.0));
    }

    #[test]
    fn given_a_rate_return_order_with_both_amounts() {
        let spread = Spread::new(0).unwrap();

        let rate = Rate::try_from(0.1).unwrap();
        let order = BtcDaiOrder::new_sell(
            btc(1051.0),
            btc(1.0),
            btc(50.0),
            None,
            rate,
            spread,
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        // 1 Sell => 0.1 Buy
        // 1000 Sell => 100 Buy
        assert_eq!(order.base, btc(1000.0));
        assert_eq!(order.quote, dai_asset(100.0));

        let rate = Rate::try_from(10.0).unwrap();
        let order = BtcDaiOrder::new_sell(
            btc(1051.0),
            btc(1.0),
            btc(50.0),
            None,
            rate,
            spread,
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        assert_eq!(order.base, btc(1000.0));
        assert_eq!(order.quote, dai_asset(10_000.0));

        let rate = Rate::try_from(0.1).unwrap();
        let order = BtcDaiOrder::new_buy(
            dai_amount(1050.0),
            dai_amount(50.0),
            None,
            rate,
            spread,
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        // 1 Sell => 0.1 Buy
        // 1000 Sell => 100 Buy
        assert_eq!(order.base, btc(100.0));
        assert_eq!(order.quote, dai_asset(1000.0));

        let rate = Rate::try_from(10.0).unwrap();
        let order = BtcDaiOrder::new_buy(
            dai_amount(1050.0),
            dai_amount(50.0),
            None,
            rate,
            spread,
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        assert_eq!(order.base, btc(10_000.0));
        assert_eq!(order.quote, dai_asset(1000.0));
    }

    #[test]
    fn given_a_rate_and_spread_return_order_with_both_amounts() {
        let rate = Rate::try_from(0.1).unwrap();
        let spread = Spread::new(300).unwrap();

        let order = BtcDaiOrder::new_sell(
            btc(1051.0),
            btc(1.0),
            btc(50.0),
            None,
            rate,
            spread,
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        assert_eq!(order.base, btc(1000.0));
        assert_eq!(order.quote, dai_asset(103.0));

        let order = BtcDaiOrder::new_buy(
            dai_amount(1051.0),
            dai_amount(51.0),
            None,
            rate,
            spread,
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        )
        .unwrap();

        assert_eq!(order.base, btc(97.0));
        assert_eq!(order.quote, dai_asset(1000.0));
    }

    #[test]
    fn given_fee_higher_than_available_funds_return_insufficient_funds() {
        let rate = Rate::try_from(1.0).unwrap();
        let spread = Spread::new(0).unwrap();

        let result = BtcDaiOrder::new_sell(
            btc(1.0),
            btc(2.0),
            btc(0.0),
            None,
            rate,
            spread,
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        );
        assert!(result.unwrap_err().downcast::<InsufficientFunds>().is_ok());

        let result = BtcDaiOrder::new_buy(
            dai_amount(1.0),
            dai_amount(2.0),
            None,
            rate,
            spread,
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        );
        assert!(result.unwrap_err().downcast::<InsufficientFunds>().is_ok());
    }

    #[test]
    fn given_reserved_funds_higher_available_funds_return_insufficient_funds() {
        let rate = Rate::try_from(1.0).unwrap();
        let spread = Spread::new(0).unwrap();

        let result = BtcDaiOrder::new_sell(
            btc(1.0),
            btc(0.0),
            btc(2.0),
            None,
            rate,
            spread,
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        );
        assert!(result.unwrap_err().downcast::<InsufficientFunds>().is_ok());

        let result = BtcDaiOrder::new_buy(
            dai_amount(1.0),
            dai_amount(2.0),
            None,
            rate,
            spread,
            DaiContractAddress::Mainnet,
            ethereum::ChainId::mainnet(),
        );
        assert!(result.unwrap_err().downcast::<InsufficientFunds>().is_ok());
    }

    #[test]
    fn sell_order_is_as_good_as_market_rate() {
        let order = BtcDaiOrder {
            position: Position::Sell,
            base: btc(1.0),
            quote: dai_asset(1.0),
        };

        let rate = MidMarketRate::new(Rate::try_from(1.0).unwrap());

        let is_profitable = order.is_as_good_as(rate).unwrap();
        assert!(is_profitable)
    }

    #[test]
    fn sell_order_is_better_than_market_rate() {
        let order = BtcDaiOrder {
            position: Position::Sell,
            base: btc(1.0),
            quote: dai_asset(1.0),
        };

        let rate = MidMarketRate::new(Rate::try_from(0.9).unwrap());

        let is_profitable = order.is_as_good_as(rate).unwrap();
        assert!(is_profitable)
    }

    #[test]
    fn sell_order_is_worse_than_market_rate() {
        let order = BtcDaiOrder {
            position: Position::Sell,
            base: btc(1.0),
            quote: dai_asset(1.0),
        };

        let rate = MidMarketRate::new(Rate::try_from(1.1).unwrap());

        let is_profitable = order.is_as_good_as(rate).unwrap();
        assert!(!is_profitable)
    }

    #[test]
    fn buy_order_is_as_good_as_market_rate() {
        let order = BtcDaiOrder {
            position: Position::Buy,
            base: btc(1.0),
            quote: dai_asset(1.0),
        };

        let rate = MidMarketRate::new(Rate::try_from(1.0).unwrap());

        let is_profitable = order.is_as_good_as(rate).unwrap();
        assert!(is_profitable)
    }

    #[test]
    fn buy_order_is_better_than_market_rate() {
        let order = BtcDaiOrder {
            position: Position::Buy,
            base: btc(1.0),
            quote: dai_asset(1.0),
        };

        let rate = MidMarketRate::new(Rate::try_from(1.1).unwrap());

        let is_profitable = order.is_as_good_as(rate).unwrap();
        assert!(is_profitable)
    }

    #[test]
    fn buy_order_is_worse_than_market_rate() {
        let order = BtcDaiOrder {
            position: Position::Buy,
            base: btc(1.0),
            quote: dai_asset(1.0),
        };

        let rate = MidMarketRate::new(Rate::try_from(0.9).unwrap());

        let is_profitable = order.is_as_good_as(rate).unwrap();
        assert!(!is_profitable)
    }

    proptest! {
        #[test]
        fn new_buy_does_not_panic(dai_balance in "[0-9]+", dai_reserved_funds in "[0-9]+", dai_max_amount in "[0-9]+", rate in any::<f64>(), spread in any::<u16>()) {

            let dai_balance = BigUint::from_str(&dai_balance);
            let dai_reserved_funds = BigUint::from_str(&dai_reserved_funds);
            let dai_max_amount = BigUint::from_str(&dai_max_amount);
            let rate = Rate::try_from(rate);
            let spread = Spread::new(spread);

            if let (Ok(dai_balance), Ok(dai_reserved_funds), Ok(dai_max_amount), Ok(rate), Ok(spread)) = (dai_balance, dai_reserved_funds, dai_max_amount, rate, spread) {
                let dai_balance = dai::Amount::from_atto(dai_balance);
                let dai_reserved_funds = dai::Amount::from_atto(dai_reserved_funds);
                let dai_max_amount = dai::Amount::from_atto(dai_max_amount);

                let _: anyhow::Result<BtcDaiOrder> = BtcDaiOrder::new_buy(dai_balance, dai_reserved_funds, Some(dai_max_amount), rate, spread, DaiContractAddress::Mainnet, ethereum::ChainId::mainnet());
            }
        }
    }

    proptest! {
        #[test]
        fn new_buy_no_max_amount_does_not_panic(dai_balance in "[0-9]+", dai_reserved_funds in "[0-9]+", rate in any::<f64>(), spread in any::<u16>()) {

            let dai_balance = BigUint::from_str(&dai_balance);
            let dai_reserved_funds = BigUint::from_str(&dai_reserved_funds);
            let rate = Rate::try_from(rate);
            let spread = Spread::new(spread);

            if let (Ok(dai_balance), Ok(dai_reserved_funds), Ok(rate), Ok(spread)) = (dai_balance, dai_reserved_funds, rate, spread) {
                let dai_balance = dai::Amount::from_atto(dai_balance);
                let dai_reserved_funds = dai::Amount::from_atto(dai_reserved_funds);

                let _: anyhow::Result<BtcDaiOrder> = BtcDaiOrder::new_buy(dai_balance, dai_reserved_funds, None, rate, spread, DaiContractAddress::Mainnet, ethereum::ChainId::mainnet());
            }
        }
    }

    proptest! {
        #[test]
        fn new_sell_does_not_panic(btc_balance in any::<u64>(), btc_fees in any::<u64>(), btc_reserved_funds in any::<u64>(), btc_max_amount in any::<u64>(), rate in any::<f64>(), spread in any::<u16>()) {

            let btc_balance = bitcoin::Amount::from_sat(btc_balance);
            let btc_fees = bitcoin::Amount::from_sat(btc_fees);
            let btc_reserved_funds = bitcoin::Amount::from_sat(btc_reserved_funds);
            let btc_max_amount = bitcoin::Amount::from_sat(btc_max_amount);
            let rate = Rate::try_from(rate);
            let spread = Spread::new(spread);

            if let (Ok(rate), Ok(spread)) = (rate, spread) {
                let _: anyhow::Result<BtcDaiOrder> = BtcDaiOrder::new_sell(btc_balance, btc_fees, btc_reserved_funds, Some(btc_max_amount), rate, spread, DaiContractAddress::Mainnet, ethereum::ChainId::mainnet());
            }
        }
    }

    proptest! {
        #[test]
        fn new_sell_no_max_amount_does_not_panic(btc_balance in any::<u64>(), btc_fees in any::<u64>(), btc_reserved_funds in any::<u64>(), rate in any::<f64>(), spread in any::<u16>()) {

            let btc_balance = bitcoin::Amount::from_sat(btc_balance);
            let btc_fees = bitcoin::Amount::from_sat(btc_fees);
            let btc_reserved_funds = bitcoin::Amount::from_sat(btc_reserved_funds);
            let rate = Rate::try_from(rate);
            let spread = Spread::new(spread);

            if let (Ok(rate), Ok(spread)) = (rate, spread) {
                let _: anyhow::Result<BtcDaiOrder> = BtcDaiOrder::new_sell(btc_balance, btc_fees, btc_reserved_funds, None, rate, spread, DaiContractAddress::Mainnet, ethereum::ChainId::mainnet());
            }
        }
    }

    #[test]
    fn symbol_serializes_correctly() {
        let btc = Symbol::Btc;
        let dai = Symbol::Dai;

        assert_eq!(String::from("BTC"), btc.to_string());
        assert_eq!(String::from("DAI"), dai.to_string());
    }
}
