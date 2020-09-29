use crate::{
    bitcoin,
    ethereum::{self, dai},
    order::{BtcDaiOrderForm, Symbol},
    MidMarketRate,
};
use comit::{ledger, order::SwapProtocol, Position, Role};

pub mod strategy;

// Bundles the state of the application
#[derive(Debug)]
pub struct Maker {
    btc_balance: Option<bitcoin::Amount>,
    dai_balance: Option<dai::Amount>,
    mid_market_rate: Option<MidMarketRate>,
    pub strategy: strategy::AllIn,
    bitcoin_network: ledger::Bitcoin,
    ethereum_chain: ethereum::Chain,
    role: Role,
    comit_network: comit::Network,
}

impl Maker {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        btc_balance: bitcoin::Amount,
        dai_balance: dai::Amount,
        mid_market_rate: MidMarketRate,
        strategy: strategy::AllIn,
        bitcoin_network: ledger::Bitcoin,
        dai_chain: ethereum::Chain,
        role: Role,
        comit_network: comit::Network,
    ) -> Self {
        Maker {
            btc_balance: Some(btc_balance),
            dai_balance: Some(dai_balance),
            mid_market_rate: Some(mid_market_rate),
            strategy,
            bitcoin_network,
            ethereum_chain: dai_chain,
            role,
            comit_network,
        }
    }

    pub fn update_rate(
        &mut self,
        mid_market_rate: MidMarketRate,
    ) -> anyhow::Result<Option<PublishOrders>> {
        match self.mid_market_rate {
            Some(previous_mid_market_rate) if previous_mid_market_rate == mid_market_rate => {
                Ok(None)
            }
            _ => {
                self.mid_market_rate = Some(mid_market_rate);

                Ok(Some(PublishOrders {
                    new_sell_order: self.new_sell_order()?,
                    new_buy_order: self.new_buy_order()?,
                }))
            }
        }
    }

    pub fn invalidate_rate(&mut self) {
        self.mid_market_rate = None;
    }

    pub fn update_bitcoin_balance(
        &mut self,
        balance: bitcoin::Amount,
    ) -> anyhow::Result<Option<BtcDaiOrderForm>> {
        // if we had a balance and the balance did not change => no new orders
        if let Some(previous_balance) = self.btc_balance {
            if previous_balance == balance {
                return Ok(None);
            }
        }

        self.btc_balance = Some(balance);
        let order = self.new_sell_order()?;
        Ok(Some(order))
    }

    pub fn invalidate_bitcoin_balance(&mut self) {
        self.btc_balance = None;
    }

    pub fn update_dai_balance(
        &mut self,
        balance: dai::Amount,
    ) -> anyhow::Result<Option<BtcDaiOrderForm>> {
        // if we had a balance and the balance did not change => no new orders
        if let Some(previous_balance) = self.dai_balance.clone() {
            if previous_balance == balance {
                return Ok(None);
            }
        }

        self.dai_balance = Some(balance);
        let order = self.new_buy_order()?;
        Ok(Some(order))
    }

    pub fn invalidate_dai_balance(&mut self) {
        self.dai_balance = None;
    }

    pub fn swap_protocol(&self, position: Position) -> SwapProtocol {
        SwapProtocol::new(self.role, position, self.comit_network)
    }

    pub fn new_sell_order(&self) -> anyhow::Result<BtcDaiOrderForm> {
        match (self.mid_market_rate, self.btc_balance) {
            (Some(mid_market_rate), Some(btc_balance)) => {
                self.strategy.new_sell(btc_balance, mid_market_rate.into())
            }
            (None, _) => anyhow::bail!(RateNotAvailable(Position::Sell)),
            (_, None) => anyhow::bail!(BalanceNotAvailable(Symbol::Btc)),
        }
    }

    pub fn new_buy_order(&self) -> anyhow::Result<BtcDaiOrderForm> {
        match (self.mid_market_rate, self.dai_balance.clone()) {
            (Some(mid_market_rate), Some(dai_balance)) => {
                self.strategy.new_buy(dai_balance, mid_market_rate.into())
            }
            (None, _) => anyhow::bail!(RateNotAvailable(Position::Buy)),
            (_, None) => anyhow::bail!(BalanceNotAvailable(Symbol::Dai)),
        }
    }

    pub async fn new_order(&self, position: Position) -> anyhow::Result<BtcDaiOrderForm> {
        match position {
            Position::Buy => self.new_buy_order(),
            Position::Sell => self.new_sell_order(),
        }
    }

    pub fn process_taken_order(
        &mut self,
        order: BtcDaiOrderForm,
    ) -> anyhow::Result<TakeRequestDecision> {
        let current_mid_market_rate = match self.mid_market_rate {
            None => anyhow::bail!(RateNotAvailable(order.position)),
            Some(rate) => rate,
        };

        let dai_balance = match self.dai_balance {
            Some(ref dai_balance) => dai_balance,
            None => anyhow::bail!(BalanceNotAvailable(Symbol::Dai)),
        };

        let btc_balance = match self.btc_balance {
            Some(ref btc_balance) => btc_balance,
            None => anyhow::bail!(BalanceNotAvailable(Symbol::Btc)),
        };

        self.strategy.process_taken_order(
            order,
            current_mid_market_rate.into(),
            dai_balance,
            btc_balance,
        )
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TakeRequestDecision {
    GoForSwap,
    RateNotProfitable,
    InsufficientFunds,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PublishOrders {
    pub new_sell_order: BtcDaiOrderForm,
    pub new_buy_order: BtcDaiOrderForm,
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("Rate not available when trying to create new {0} order.")]
pub struct RateNotAvailable(Position);

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("{0} balance not available.")]
pub struct BalanceNotAvailable(Symbol);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bitcoin,
        bitcoin::amount::{btc, some_btc},
        ethereum::dai::{dai, some_dai},
        order::{btc_dai_order_form, BtcDaiOrderForm},
        rate::rate,
        MidMarketRate, Rate, Spread, StaticStub,
    };
    use std::convert::TryFrom;

    impl StaticStub for Maker {
        fn static_stub() -> Self {
            Self {
                btc_balance: Some(bitcoin::Amount::default()),
                dai_balance: Some(dai::Amount::default()),
                strategy: strategy::AllIn::static_stub(),
                mid_market_rate: Some(MidMarketRate::static_stub()),
                bitcoin_network: ledger::Bitcoin::Mainnet,
                ethereum_chain: ethereum::Chain::static_stub(),
                role: Role::Bob,
                comit_network: comit::Network::Main,
            }
        }
    }

    fn some_rate(rate: f64) -> Option<MidMarketRate> {
        Some(MidMarketRate::new(Rate::try_from(rate).unwrap()))
    }

    #[test]
    fn yield_error_if_rate_is_not_available() {
        let mut maker = Maker {
            mid_market_rate: None,
            ..StaticStub::static_stub()
        };

        let taken_order = BtcDaiOrderForm {
            ..StaticStub::static_stub()
        };

        let result = maker.process_taken_order(taken_order);
        assert!(result.is_err());

        let result = maker.new_buy_order();
        assert!(result.is_err());

        let result = maker.new_sell_order();
        assert!(result.is_err());
    }

    #[test]
    fn fail_to_confirm_sell_order_if_sell_rate_is_not_good_enough() {
        let mut maker = Maker {
            mid_market_rate: some_rate(10000.0),
            ..StaticStub::static_stub()
        };

        let taken_order = btc_dai_order_form(Position::Sell, btc(1.0), rate(9000.0));

        let result = maker.process_taken_order(taken_order).unwrap();

        assert_eq!(result, TakeRequestDecision::RateNotProfitable);
    }

    #[test]
    fn fail_to_confirm_buy_order_if_buy_rate_is_not_good_enough() {
        let mut maker = Maker {
            mid_market_rate: some_rate(10000.0),
            ..StaticStub::static_stub()
        };

        let taken_order = btc_dai_order_form(Position::Buy, btc(1.0), rate(11000.0));

        let result = maker.process_taken_order(taken_order).unwrap();

        assert_eq!(result, TakeRequestDecision::RateNotProfitable);
    }

    #[test]
    fn no_rate_change_if_rate_update_with_same_value() {
        let init_rate = some_rate(1.0);
        let mut maker = Maker {
            mid_market_rate: init_rate,
            ..StaticStub::static_stub()
        };

        let new_mid_market_rate = MidMarketRate::new(Rate::try_from(1.0).unwrap());

        let result = maker.update_rate(new_mid_market_rate).unwrap();
        assert!(result.is_none());
        assert_eq!(maker.mid_market_rate, init_rate)
    }

    #[test]
    fn rate_updated_and_new_orders_if_rate_update_with_new_value() {
        let mut maker = Maker {
            btc_balance: some_btc(10.0),
            dai_balance: some_dai(10.0),
            mid_market_rate: some_rate(1.0),
            ..StaticStub::static_stub()
        };

        let new_mid_market_rate = MidMarketRate::new(Rate::try_from(2.0).unwrap());

        let result = maker.update_rate(new_mid_market_rate).unwrap();
        assert!(result.is_some());
        assert_eq!(maker.mid_market_rate, Some(new_mid_market_rate))
    }

    #[test]
    fn no_new_sell_order_if_no_btc_balance_change() {
        let mut maker = Maker {
            btc_balance: some_btc(1.0),
            ..StaticStub::static_stub()
        };

        let result = maker.update_bitcoin_balance(btc(1.0)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn no_new_buy_order_if_no_dai_balance_change() {
        let mut maker = Maker {
            dai_balance: some_dai(1.0),
            ..StaticStub::static_stub()
        };

        let result = maker.update_dai_balance(dai(1.0)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn new_sell_order_if_btc_balance_change() {
        let mut maker = Maker {
            btc_balance: some_btc(1.0),
            mid_market_rate: some_rate(1.0),
            ..StaticStub::static_stub()
        };
        let new_balance = btc(0.5);

        let new_sell_order = maker.update_bitcoin_balance(new_balance).unwrap().unwrap();
        assert_eq!(new_sell_order.position, Position::Sell);
        assert_eq!(maker.btc_balance, Some(new_balance))
    }

    #[test]
    fn new_buy_order_if_dai_balance_change() {
        let mut maker = Maker {
            dai_balance: some_dai(1.0),
            mid_market_rate: some_rate(1.0),
            ..StaticStub::static_stub()
        };
        let new_balance = dai(0.5);

        let new_buy_order = maker
            .update_dai_balance(new_balance.clone())
            .unwrap()
            .unwrap();
        assert_eq!(new_buy_order.position, Position::Buy);
        assert_eq!(maker.dai_balance, Some(new_balance))
    }

    #[test]
    fn published_sell_order_can_be_taken() {
        let strategy = strategy::AllIn::new(
            Default::default(),
            btc(0.0),
            None,
            Some(btc(1.0)),
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let mut maker = Maker {
            btc_balance: some_btc(3.0),
            mid_market_rate: some_rate(1.0),
            strategy,
            ..StaticStub::static_stub()
        };

        let new_sell_order = maker.new_sell_order().unwrap();
        assert_eq!(new_sell_order.quantity.sats(), btc(1.0).as_sat());

        let result = maker.process_taken_order(new_sell_order).unwrap();

        assert_eq!(result, TakeRequestDecision::GoForSwap);
    }

    #[test]
    fn published_buy_order_can_be_taken() {
        let strategy = strategy::AllIn::new(
            Default::default(),
            btc(0.0),
            Some(btc(1.0)),
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let mut maker = Maker {
            dai_balance: some_dai(3.0),
            mid_market_rate: some_rate(1.0),
            strategy,
            ..StaticStub::static_stub()
        };

        let new_buy_order = maker.new_buy_order().unwrap();
        assert_eq!(dai::Amount::from(new_buy_order.quote()), dai(1.0));

        let result = maker.process_taken_order(new_buy_order).unwrap();

        assert_eq!(result, TakeRequestDecision::GoForSwap);
    }

    #[test]
    fn new_buy_order_with_max_buy() {
        let strategy = strategy::AllIn::new(
            Default::default(),
            btc(0.0),
            Some(btc(0.002)),
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let maker = Maker {
            dai_balance: some_dai(20.0),
            mid_market_rate: some_rate(9000.0),
            btc_balance: some_btc(1.0),
            strategy,
            ..StaticStub::static_stub()
        };

        let new_buy_order = maker.new_buy_order().unwrap();
        assert_eq!(new_buy_order.quantity.to_inner(), btc(0.002));
        assert_eq!(dai::Amount::from(new_buy_order.quote()), dai(18.0));
    }

    #[test]
    fn new_buy_order() {
        let strategy = strategy::AllIn::new(
            Default::default(),
            btc(0.0),
            None,
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let maker = Maker {
            dai_balance: some_dai(20.0),
            mid_market_rate: some_rate(10000.0),
            btc_balance: some_btc(1.0),
            strategy,
            ..StaticStub::static_stub()
        };

        let new_buy_order = maker.new_buy_order().unwrap();
        assert_eq!(new_buy_order.quantity.to_inner(), btc(0.002));
        assert_eq!(dai::Amount::from(new_buy_order.quote()), dai(20.0));
    }
}
