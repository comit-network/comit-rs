use crate::{
    bitcoin,
    bitcoin::Fee,
    config,
    ethereum::dai,
    maker::TakeRequestDecision,
    order::{BtcDaiOrderForm, Symbol},
    swap::SwapKind,
    Rate, Result, Spread,
};
use anyhow::anyhow;
use comit::{BtcDaiOrder, Position, Quantity};
use std::cmp::min;

/// Create orders with the full balance, capped by a configuration setting.
/// A spread is applied on the passed mid-market rate
#[derive(Debug)]
pub struct AllIn {
    bitcoin_fee: Fee,
    btc_reserved_funds: bitcoin::Amount,
    dai_reserved_funds: dai::Amount,
    max_buy_quantity: Option<bitcoin::Amount>,
    max_sell_quantity: Option<bitcoin::Amount>,
    spread: Spread,
}

impl AllIn {
    pub fn new(
        btc_fee_strategy: config::BitcoinFeeStrategy,
        max_buy_quantity: Option<bitcoin::Amount>,
        max_sell_quantity: Option<bitcoin::Amount>,
        spread: Spread,
        bitcoind_client: bitcoin::Client,
    ) -> Self {
        let bitcoin_fee = Fee::new(btc_fee_strategy, bitcoind_client);
        Self {
            bitcoin_fee,
            btc_reserved_funds: Default::default(),
            dai_reserved_funds: Default::default(),
            max_buy_quantity,
            max_sell_quantity,
            spread,
        }
    }
}

// Methods that are likely to be in the `Strategy` trait
impl AllIn {
    /// Inform the strategy that a hbit_herc20 swap execution was resumed
    pub fn hbit_herc20_swap_resumed(&mut self, fund_amount: dai::Amount) {
        self.dai_reserved_funds += fund_amount;
    }

    /// Inform the strategy that a herc20_hbit swap execution was resumed
    pub fn herc20_hbit_swap_resumed(&mut self, fund_amount: bitcoin::Amount) -> Result<()> {
        let amount_to_reserve = fund_amount
            .checked_add(self.bitcoin_fee.max_tx_fee())
            .ok_or_else(|| anyhow!(Overflow))?;

        self.btc_reserved_funds = self
            .btc_reserved_funds
            .checked_add(amount_to_reserve)
            .ok_or_else(|| anyhow!(Overflow))?;

        Ok(())
    }

    /// Create a new sell order given the passed parameters.
    /// Bitcoin is always the base asset, sell order sells bitcoin.
    ///
    /// The quantity is the full available balance minus the expected mining fee
    /// or the max btc sell parameters, whichever is the lowest.
    /// The spread parameter is applied on the mid market rate to decide
    /// the price.
    pub fn new_sell(
        &self,
        base_balance: bitcoin::Amount,
        mid_market_rate: Rate,
    ) -> Result<BtcDaiOrderForm> {
        match self
            .btc_reserved_funds
            .checked_add(self.bitcoin_fee.max_tx_fee())
        {
            Some(added) => {
                if base_balance <= added {
                    anyhow::bail!(InsufficientFunds(Symbol::Btc))
                }
            }
            None => anyhow::bail!(Overflow),
        }

        let base_amount = match self.max_sell_quantity {
            Some(max_quantity) => min(base_balance - self.btc_reserved_funds, max_quantity),
            None => base_balance - self.btc_reserved_funds,
        };

        let rate = self.spread.apply(mid_market_rate, Position::Sell)?;

        Ok(BtcDaiOrderForm {
            position: Position::Sell,
            quantity: Quantity::new(base_amount),
            price: rate.into(),
        })
    }

    /// Create a new buy order given the passed parameters.
    /// Bitcoin is always the base asset, buy order buys bitcoin.
    ///
    /// The quantity is the full available dai balance in btc given the current
    /// rate or the maximum buy quantity parameter, whichever is the
    /// lowest. The spread parameter is applied on the mid market rate to
    /// decide the price.
    pub fn new_buy(
        &self,
        quote_balance: dai::Amount,
        mid_market_rate: Rate,
    ) -> Result<BtcDaiOrderForm> {
        if quote_balance <= self.dai_reserved_funds {
            anyhow::bail!(InsufficientFunds(Symbol::Dai))
        }

        let rate = self.spread.apply(mid_market_rate, Position::Buy)?;
        let max_quote = quote_balance - self.dai_reserved_funds.clone();
        let max_quote_worth_in_base = max_quote.worth_in(rate)?;

        let base_amount = match self.max_buy_quantity {
            Some(max_quantity) => min(max_quote_worth_in_base, max_quantity),
            None => max_quote_worth_in_base,
        };

        Ok(BtcDaiOrderForm {
            position: Position::Buy,
            quantity: Quantity::new(base_amount),
            price: rate.into(),
        })
    }

    /// Decide whether we should proceed with an order,
    /// Checks:
    /// - funds are available
    /// - Order is considered profitable
    /// - Reserve the funds (assumes we proceed with the order)
    /// // TODO: extract the reserve part and expect consumer to call
    /// `hbit_herc20_swap_resumed` when a swap starts.
    pub fn process_taken_order(
        &mut self,
        order: BtcDaiOrder,
        current_mid_market_rate: Rate,
        dai_balance: &dai::Amount,
        btc_balance: &bitcoin::Amount,
    ) -> anyhow::Result<TakeRequestDecision> {
        let current_profitable_rate = self.spread.apply(current_mid_market_rate, order.position)?;

        if !is_as_profitable_as(&order, current_profitable_rate) {
            return Ok(TakeRequestDecision::RateNotProfitable);
        }

        match order.position {
            Position::Buy => {
                let updated_dai_reserved_funds =
                    self.dai_reserved_funds.clone() + dai::Amount::from(order.quote());
                if updated_dai_reserved_funds > *dai_balance {
                    return Ok(TakeRequestDecision::InsufficientFunds);
                }

                self.dai_reserved_funds = updated_dai_reserved_funds;
            }
            Position::Sell => {
                let updated_btc_reserved_funds = self.btc_reserved_funds
                    + order.quantity.to_inner()
                    + self.bitcoin_fee.max_tx_fee();
                if updated_btc_reserved_funds > *btc_balance {
                    return Ok(TakeRequestDecision::InsufficientFunds);
                }

                self.btc_reserved_funds = updated_btc_reserved_funds;
            }
        };

        Ok(TakeRequestDecision::GoForSwap)
    }

    /// Process a finished swap.
    pub fn swap_finished(&mut self, swap: SwapKind) {
        match swap {
            SwapKind::Herc20Hbit(swap) => {
                self.btc_reserved_funds -=
                    swap.hbit_params.shared.asset + self.bitcoin_fee.max_tx_fee();
            }
            SwapKind::HbitHerc20(swap) => {
                self.dai_reserved_funds -= swap.herc20_params.asset.into();
            }
        }
    }
}

fn is_as_profitable_as(order: &BtcDaiOrder, profitable_rate: Rate) -> bool {
    match order.position {
        Position::Buy => {
            // We are buying BTC for DAI
            // Given an order rate of: 1:9000
            // It is NOT profitable to buy, if the current rate is greater than the order
            // rate. 1:8800 -> We give less DAI for getting BTC -> Good.
            // 1:9200 -> We have to give more DAI for getting BTC -> Sucks.
            order.price <= profitable_rate.into()
        }
        Position::Sell => {
            // We are selling BTC for DAI
            // Given an order rate of: 1:9000
            // It is NOT profitable to sell, if the current rate is smaller than the order
            // rate. 1:8800 -> We get less DAI for our BTC -> Sucks.
            // 1:9200 -> We get more DAI for our BTC -> Good.
            order.price >= profitable_rate.into()
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

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("{0} balance not available.")]
pub struct BalanceNotAvailable(Symbol);

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        bitcoin::amount::btc, config, config::BitcoinFeeStrategy, ethereum::dai::dai,
        order::btc_dai_order, rate::rate, MidMarketRate, StaticStub,
    };
    use num::BigUint;
    use proptest::prelude::*;
    use std::{convert::TryFrom, str::FromStr};

    impl StaticStub for AllIn {
        fn static_stub() -> Self {
            AllIn::new(
                Default::default(),
                None,
                None,
                StaticStub::static_stub(),
                StaticStub::static_stub(),
            )
        }
    }

    #[test]
    fn given_fee_higher_than_available_funds_return_insufficient_funds() {
        let rate = Rate::try_from(1.0).unwrap();
        let spread = Spread::new(0).unwrap();

        let btc_fee_strategy = config::BitcoinFeeStrategy::SatsPerByte(btc(0.001));

        let strategy = AllIn::new(
            btc_fee_strategy,
            None,
            None,
            spread,
            StaticStub::static_stub(),
        );

        let result = strategy.new_sell(btc(0.07), rate);
        assert!(result.unwrap_err().downcast::<InsufficientFunds>().is_ok());
    }

    #[test]
    fn given_reserved_funds_higher_available_funds_return_insufficient_funds() {
        let rate = Rate::try_from(1.0).unwrap();

        let mut strategy = AllIn::static_stub();

        // Resuming a swap should take some reserve for the swap amount and fee.
        strategy.herc20_hbit_swap_resumed(btc(1.0)).unwrap();
        strategy.hbit_herc20_swap_resumed(dai(1.0));

        let result = strategy.new_sell(btc(1.0), rate);
        assert!(result.unwrap_err().downcast::<InsufficientFunds>().is_ok());

        let result = strategy.new_buy(dai(1.0), rate);
        assert!(result.unwrap_err().downcast::<InsufficientFunds>().is_ok());
    }

    #[test]
    fn given_a_balance_return_order_selling_full_balance() {
        let strategy = AllIn::static_stub();

        let rate = Rate::try_from(1.0).unwrap();
        let order = strategy.new_sell(btc(10.0), rate).unwrap();

        assert_eq!(order.quantity.to_inner(), btc(10.0));

        let order = strategy.new_buy(dai(10.0), rate).unwrap();

        assert_eq!(dai::Amount::from(order.quote()), dai(10.0));
    }

    #[test]
    fn given_a_balance_and_locked_funds_return_order_selling_available_balance() {
        let rate = Rate::try_from(1.0).unwrap();
        let mut strategy = AllIn::static_stub();

        // Resuming a swap should take some reserve.
        strategy.herc20_hbit_swap_resumed(btc(2.0)).unwrap();
        strategy.hbit_herc20_swap_resumed(dai(2.0));

        let order = strategy.new_sell(btc(10.0), rate).unwrap();

        // 35 sat * 1000 fees.
        assert_eq!(order.quantity.to_inner(), btc(7.9997));

        let order = strategy.new_buy(dai(10.0), rate).unwrap();

        assert_eq!(dai::Amount::from(order.quote()), dai(8.0));
    }

    #[test]
    fn given_an_available_balance_and_a_max_quantity_sell_min_of_either() {
        let rate = Rate::try_from(1.0).unwrap();
        let strategy = AllIn::new(
            Default::default(),
            Some(btc(2.0)),
            Some(btc(2.0)),
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let order = strategy.new_sell(btc(10.0), rate).unwrap();

        assert_eq!(order.quantity.to_inner(), btc(2.0));

        let order = strategy.new_buy(dai(10.0), rate).unwrap();

        assert_eq!(dai::Amount::from(order.quote()), dai(2.0));
    }

    #[test]
    fn given_an_available_balance_and_fees_sell_balance() {
        let rate = Rate::try_from(1.0).unwrap();
        let strategy = AllIn::new(
            Default::default(),
            None,
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let order = strategy.new_sell(btc(10.0), rate).unwrap();

        assert_eq!(order.quantity.to_inner(), btc(10.0));
    }

    #[test]
    fn given_balance_is_fees_sell_order_fails() {
        let rate = Rate::try_from(1.0).unwrap();
        let btc_fee_strategy = config::BitcoinFeeStrategy::SatsPerByte(btc(0.1));

        let strategy = AllIn::new(
            btc_fee_strategy,
            None,
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let result = strategy.new_sell(btc(1.0), rate);

        assert!(result.unwrap_err().downcast::<InsufficientFunds>().is_ok());
    }

    #[test]
    fn given_balance_is_less_than_fees_sell_order_fails() {
        let rate = Rate::try_from(1.0).unwrap();

        let btc_fee_strategy = BitcoinFeeStrategy::SatsPerByte(bitcoin::Amount::from_sat(10000));

        let strategy = AllIn::new(
            btc_fee_strategy,
            None,
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let result = strategy.new_sell(btc(0.07), rate);

        assert!(result.unwrap_err().downcast::<InsufficientFunds>().is_ok());
    }

    #[test]
    fn given_a_rate_return_order_with_both_amounts() {
        let spread = Spread::new(0).unwrap();
        let mut strategy = AllIn::new(
            StaticStub::static_stub(),
            None,
            None,
            spread,
            StaticStub::static_stub(),
        );

        // Resuming a swap should take some reserve for the swap amount and fee.
        strategy.herc20_hbit_swap_resumed(btc(50.0)).unwrap();
        strategy.hbit_herc20_swap_resumed(dai(50.0));

        let rate = Rate::try_from(0.1).unwrap();
        let order = strategy.new_sell(btc(1050.0), rate).unwrap();

        assert_eq!(order.quantity.to_inner(), btc(1000.0));
        assert_eq!(dai::Amount::from(order.quote()), dai(100.0));

        let rate = Rate::try_from(10.0).unwrap();
        let order = strategy.new_sell(btc(1050.0), rate).unwrap();

        assert_eq!(order.quantity.to_inner(), btc(1000.0));
        assert_eq!(dai::Amount::from(order.quote()), dai(10_000.0));

        let rate = Rate::try_from(0.1).unwrap();
        let order = strategy.new_buy(dai(1050.0), rate).unwrap();

        assert_eq!(order.quantity.to_inner(), btc(10_000.0));
        assert_eq!(dai::Amount::from(order.quote()), dai(1000.0));

        let rate = Rate::try_from(10.0).unwrap();
        let order = strategy.new_buy(dai(1050.0), rate).unwrap();

        assert_eq!(order.quantity.to_inner(), btc(100.0));
        assert_eq!(dai::Amount::from(order.quote()), dai(1000.0));
    }

    #[test]
    fn given_a_rate_and_spread_return_order_with_both_amounts_correct_1() {
        let rate = Rate::try_from(10_000.0).unwrap();
        let spread = Spread::new(300).unwrap();
        let mut strategy = AllIn::new(
            StaticStub::static_stub(),
            None,
            None,
            spread,
            StaticStub::static_stub(),
        );

        assert_eq!(
            spread.apply(rate, Position::Sell).unwrap().integer(),
            BigUint::from(103000000000000 as u64)
        );

        // Resuming a swap should take some reserve for the swap amount and fee.
        strategy.herc20_hbit_swap_resumed(btc(0.5)).unwrap();
        strategy.hbit_herc20_swap_resumed(dai(51.0));

        let order = strategy.new_sell(btc(1.5), rate).unwrap();

        assert_eq!(order.quantity.to_inner(), btc(1.0));
        assert_eq!(dai::Amount::from(order.quote()), dai(10_300.0));

        assert_eq!(
            spread.apply(rate, Position::Buy).unwrap().integer(),
            BigUint::from(97000000000000 as u64)
        );

        let order = strategy.new_buy(dai(10_051.0), rate).unwrap();

        assert_eq!(order.quantity.to_inner(), btc(1.03092783));
        assert_eq!(dai::Amount::from(order.quote()), dai(9999.999951));
    }

    #[test]
    fn btc_funds_reserved_upon_taking_sell_order() {
        let mut strategy = AllIn::new(
            BitcoinFeeStrategy::default(),
            None,
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let taken_order = btc_dai_order(Position::Sell, btc(1.5), rate(0.0));

        let event = strategy
            .process_taken_order(taken_order, Rate::static_stub(), &dai(0.0), &btc(3.0))
            .unwrap();

        assert_eq!(event, TakeRequestDecision::GoForSwap);
        assert_eq!(strategy.btc_reserved_funds, btc(1.5003))
    }

    proptest! {
        #[test]
        fn new_buy_does_not_panic(dai_balance in "[0-9]+", max_buy_quantity in any::<u64>(), rate in any::<f64>(), spread in any::<u16>()) {

            let max_buy_quantity = bitcoin::Amount::from_sat(max_buy_quantity);
            let dai_balance = BigUint::from_str(&dai_balance);
            let rate = Rate::try_from(rate);
            let spread = Spread::new(spread);

            if let (Ok(dai_balance), Ok(rate), Ok(spread)) = (dai_balance, rate, spread) {
                let dai_balance = dai::Amount::from_atto(dai_balance);

                let strategy = AllIn::new(Default::default(), None, Some(max_buy_quantity), spread, StaticStub::static_stub(),);
                let _: anyhow::Result<BtcDaiOrderForm> = strategy.new_buy(dai_balance, rate);
            }
        }
    }

    proptest! {
        #[test]
        fn new_buy_no_max_quantity_does_not_panic(dai_balance in "[0-9]+", rate in any::<f64>(), spread in any::<u16>()) {

            let dai_balance = BigUint::from_str(&dai_balance);
            let rate = Rate::try_from(rate);
            let spread = Spread::new(spread);

            if let (Ok(dai_balance), Ok(rate), Ok(spread)) = (dai_balance, rate, spread) {
                let strategy = AllIn::new(Default::default(), None, None, spread, StaticStub::static_stub(),);

                let dai_balance = dai::Amount::from_atto(dai_balance);

                let _: anyhow::Result<BtcDaiOrderForm> = strategy.new_buy(dai_balance, rate);
            }
        }
    }

    proptest! {
        #[test]
        fn new_sell_does_not_panic(btc_balance in any::<u64>(), max_sell_quantity in any::<u64>(), rate in any::<f64>(), spread in any::<u16>()) {

            let btc_balance = bitcoin::Amount::from_sat(btc_balance);
            let max_sell_quantity = bitcoin::Amount::from_sat(max_sell_quantity);
            let rate = Rate::try_from(rate);
            let spread = Spread::new(spread);

            if let (Ok(rate), Ok(spread)) = (rate, spread) {
                let strategy = AllIn::new(Default::default(), Some(max_sell_quantity), None, spread, StaticStub::static_stub());

                let _: anyhow::Result<BtcDaiOrderForm> = strategy.new_sell(btc_balance, rate);
            }
        }
    }

    proptest! {
        #[test]
        fn new_sell_no_max_quantity_does_not_panic(btc_balance in any::<u64>(), rate in any::<f64>(), spread in any::<u16>()) {

            let btc_balance = bitcoin::Amount::from_sat(btc_balance);
            let rate = Rate::try_from(rate);
            let spread = Spread::new(spread);

            if let (Ok(rate), Ok(spread)) = (rate, spread) {
                let strategy = AllIn::new(Default::default(), None, None, spread, StaticStub::static_stub());

                let _: anyhow::Result<BtcDaiOrderForm> = strategy.new_sell(btc_balance, rate);
            }
        }
    }

    #[test]
    fn btc_funds_reserved_upon_taking_sell_order_with_fee() {
        let btc_fee_strategy = BitcoinFeeStrategy::SatsPerByte(bitcoin::Amount::from_sat(1000));

        let mut strategy = AllIn::new(
            btc_fee_strategy,
            None,
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let taken_order = btc_dai_order(Position::Sell, btc(1.5), rate(0.0));

        let event = strategy
            .process_taken_order(taken_order, Rate::static_stub(), &dai(0.0), &btc(3.0))
            .unwrap();

        assert_eq!(event, TakeRequestDecision::GoForSwap);
        assert_eq!(strategy.btc_reserved_funds, btc(1.53))
    }

    #[test]
    fn dai_funds_reserved_upon_taking_buy_order() {
        let mut strategy = AllIn::new(
            Default::default(),
            None,
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let taken_order = btc_dai_order(Position::Buy, btc(1.0), rate(1.5));

        let result = strategy
            .process_taken_order(taken_order, rate(1.5), &dai(10000.0), &btc(0.0))
            .unwrap();

        assert_eq!(result, TakeRequestDecision::GoForSwap);
        assert_eq!(strategy.dai_reserved_funds, dai(1.5))
    }

    #[test]
    fn dai_funds_reserved_upon_taking_buy_order_with_fee() {
        let mut strategy = AllIn::new(
            Default::default(),
            None,
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let taken_order = btc_dai_order(Position::Buy, btc(1.0), rate(1.5));

        let result = strategy
            .process_taken_order(taken_order, rate(1.5), &dai(10000.0), &btc(0.0))
            .unwrap();

        assert_eq!(result, TakeRequestDecision::GoForSwap);
        assert_eq!(strategy.dai_reserved_funds, dai(1.5))
    }

    #[test]
    fn not_enough_btc_funds_to_reserve_for_a_sell_order() {
        let mut strategy = AllIn::new(
            Default::default(),
            None,
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let taken_order = btc_dai_order(Position::Sell, btc(1.5), rate(0.0));

        let result = strategy
            .process_taken_order(taken_order, Rate::static_stub(), &dai(0.1), &btc(0.1))
            .unwrap();

        assert_eq!(result, TakeRequestDecision::InsufficientFunds);
    }

    #[test]
    fn not_enough_btc_funds_to_reserve_for_a_buy_order() {
        let mut strategy = AllIn::new(
            Default::default(),
            None,
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let taken_order = btc_dai_order(Position::Buy, btc(1.0), rate(1.5));

        let result = strategy
            .process_taken_order(taken_order, rate(1.5), &dai(0.0), &btc(0.0))
            .unwrap();

        assert_eq!(result, TakeRequestDecision::InsufficientFunds);
    }

    #[test]
    fn sell_order_is_as_good_as_market_rate() {
        let order = btc_dai_order(Position::Sell, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(1.0).unwrap());

        let is_profitable = is_as_profitable_as(&order, rate.into());
        assert!(is_profitable)
    }

    #[test]
    fn sell_order_is_better_than_market_rate() {
        let order = btc_dai_order(Position::Sell, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(0.9).unwrap());

        let is_profitable = is_as_profitable_as(&order, rate.into());
        assert!(is_profitable)
    }

    #[test]
    fn sell_order_is_worse_than_market_rate() {
        let order = btc_dai_order(Position::Sell, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(1.1).unwrap());

        let is_profitable = is_as_profitable_as(&order, rate.into());
        assert!(!is_profitable)
    }

    #[test]
    fn buy_order_is_as_good_as_market_rate() {
        let order = btc_dai_order(Position::Buy, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(1.0).unwrap());

        let is_profitable = is_as_profitable_as(&order, rate.into());
        assert!(is_profitable)
    }

    #[test]
    fn buy_order_is_better_than_market_rate() {
        let order = btc_dai_order(Position::Buy, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(1.1).unwrap());

        let is_profitable = is_as_profitable_as(&order, rate.into());
        assert!(is_profitable)
    }

    #[test]
    fn buy_order_is_worse_than_market_rate() {
        let order = btc_dai_order(Position::Buy, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(0.9).unwrap());

        let is_profitable = is_as_profitable_as(&order, rate.into());
        assert!(!is_profitable)
    }
}
