use crate::Rate;
use comit::{
    asset::{Bitcoin, Erc20Quantity},
    order::SwapProtocol,
    Position, Price, Quantity,
};

#[derive(Debug, Copy, Clone, strum_macros::Display)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Symbol {
    Btc,
    Dai,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtcDaiOrderForm {
    pub position: Position,
    pub quantity: Quantity<Bitcoin>,
    pub price: Price<Bitcoin, Erc20Quantity>,
}

impl BtcDaiOrderForm {
    pub fn to_comit_order(&self, swap_protocol: SwapProtocol) -> comit::BtcDaiOrder {
        comit::BtcDaiOrder::new(
            self.position,
            self.quantity,
            self.price.clone(),
            swap_protocol,
        )
    }

    pub fn quote(&self) -> Erc20Quantity {
        self.quantity * self.price.clone()
    }

    pub fn is_as_profitable_as(&self, profitable_rate: Rate) -> anyhow::Result<bool> {
        let order_rate = self.price.clone();
        match self.position {
            Position::Buy => {
                // We are buying BTC for DAI
                // Given an order rate of: 1:9000
                // It is NOT profitable to buy, if the current rate is greater than the order
                // rate. 1:8800 -> We give less DAI for getting BTC -> Good.
                // 1:9200 -> We have to give more DAI for getting BTC -> Sucks.
                Ok(order_rate <= profitable_rate.into())
            }
            Position::Sell => {
                // We are selling BTC for DAI
                // Given an order rate of: 1:9000
                // It is NOT profitable to sell, if the current rate is smaller than the order
                // rate. 1:8800 -> We get less DAI for our BTC -> Sucks.
                // 1:9200 -> We get more DAI for our BTC -> Good.
                Ok(order_rate >= profitable_rate.into())
            }
        }
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
impl crate::StaticStub for BtcDaiOrderForm {
    fn static_stub() -> Self {
        use std::convert::TryFrom;
        Self {
            position: Position::Buy,
            quantity: Quantity::new(Bitcoin::from_sat(1)),
            price: Rate::try_from(1.0).unwrap().into(),
        }
    }
}

#[cfg(test)]
pub fn btc_dai_order_form(
    position: Position,
    btc_quantity: bitcoin::Amount,
    btc_dai_rate: Rate,
) -> BtcDaiOrderForm {
    BtcDaiOrderForm {
        position,
        quantity: Quantity::new(btc_quantity),
        price: btc_dai_rate.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bitcoin::amount::btc, rate::rate, MidMarketRate, Rate};
    use std::convert::TryFrom;

    #[test]
    fn sell_order_is_as_good_as_market_rate() {
        let order = btc_dai_order_form(Position::Sell, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(1.0).unwrap());

        let is_profitable = order.is_as_profitable_as(rate.into()).unwrap();
        assert!(is_profitable)
    }

    #[test]
    fn sell_order_is_better_than_market_rate() {
        let order = btc_dai_order_form(Position::Sell, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(0.9).unwrap());

        let is_profitable = order.is_as_profitable_as(rate.into()).unwrap();
        assert!(is_profitable)
    }

    #[test]
    fn sell_order_is_worse_than_market_rate() {
        let order = btc_dai_order_form(Position::Sell, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(1.1).unwrap());

        let is_profitable = order.is_as_profitable_as(rate.into()).unwrap();
        assert!(!is_profitable)
    }

    #[test]
    fn buy_order_is_as_good_as_market_rate() {
        let order = btc_dai_order_form(Position::Buy, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(1.0).unwrap());

        let is_profitable = order.is_as_profitable_as(rate.into()).unwrap();
        assert!(is_profitable)
    }

    #[test]
    fn buy_order_is_better_than_market_rate() {
        let order = btc_dai_order_form(Position::Buy, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(1.1).unwrap());

        let is_profitable = order.is_as_profitable_as(rate.into()).unwrap();
        assert!(is_profitable)
    }

    #[test]
    fn buy_order_is_worse_than_market_rate() {
        let order = btc_dai_order_form(Position::Buy, btc(1.0), rate(1.0));

        let rate = MidMarketRate::new(Rate::try_from(0.9).unwrap());

        let is_profitable = order.is_as_profitable_as(rate.into()).unwrap();
        assert!(!is_profitable)
    }

    #[test]
    fn symbol_serializes_correctly() {
        let btc = Symbol::Btc;
        let dai = Symbol::Dai;

        assert_eq!(String::from("BTC"), btc.to_string());
        assert_eq!(String::from("DAI"), dai.to_string());
    }
}
