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
            price: crate::Rate::try_from(1.0).unwrap().into(),
        }
    }
}

#[cfg(test)]
impl crate::StaticStub for comit::BtcDaiOrder {
    fn static_stub() -> Self {
        use std::convert::TryFrom;
        Self {
            id: "7e0a0846-9765-4221-adf9-cda739a998d2".parse().unwrap(),
            position: Position::Buy,
            swap_protocol: SwapProtocol::new(
                comit::Role::Alice,
                Position::Buy,
                comit::Network::Dev,
            ),
            created_at: time::OffsetDateTime::from_unix_timestamp(0),
            quantity: Quantity::new(Bitcoin::from_sat(1)),
            price: crate::Rate::try_from(1.0).unwrap().into(),
        }
    }
}

#[cfg(test)]
pub fn btc_dai_order(
    position: Position,
    btc_quantity: bitcoin::Amount,
    btc_dai_rate: crate::Rate,
) -> comit::BtcDaiOrder {
    comit::BtcDaiOrder {
        id: comit::OrderId::random(),
        position,
        swap_protocol: SwapProtocol::new(comit::Role::Alice, Position::Buy, comit::Network::Dev),
        created_at: time::OffsetDateTime::from_unix_timestamp(0),
        quantity: Quantity::new(btc_quantity),
        price: btc_dai_rate.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_serializes_correctly() {
        let btc = Symbol::Btc;
        let dai = Symbol::Dai;

        assert_eq!(String::from("BTC"), btc.to_string());
        assert_eq!(String::from("DAI"), dai.to_string());
    }
}
