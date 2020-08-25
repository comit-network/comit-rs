use crate::{asset, asset::Erc20Quantity, Role};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrderId(Uuid);

impl OrderId {
    pub fn random() -> OrderId {
        OrderId(Uuid::new_v4())
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for OrderId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::from_str(s)?;
        Ok(OrderId(uuid))
    }
}

impl From<Uuid> for OrderId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BtcDaiOrder {
    pub id: OrderId,
    pub position: Position,
    pub swap_protocol: SwapProtocol,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    pub quantity: asset::Bitcoin,
    /// The price of this order in WEI per SATOSHI.
    price: Erc20Quantity,
}

impl BtcDaiOrder {
    pub fn buy(
        quantity: asset::Bitcoin,
        price: Erc20Quantity,
        swap_protocol: SwapProtocol,
    ) -> Self {
        Self::new(Position::Buy, quantity, price, swap_protocol)
    }

    pub fn sell(
        quantity: asset::Bitcoin,
        price: Erc20Quantity,
        swap_protocol: SwapProtocol,
    ) -> Self {
        Self::new(Position::Sell, quantity, price, swap_protocol)
    }

    pub fn new(
        position: Position,
        quantity: asset::Bitcoin,
        price: Erc20Quantity,
        swap_protocol: SwapProtocol,
    ) -> BtcDaiOrder {
        Self {
            id: OrderId::random(),
            position,
            quantity,
            price,
            swap_protocol,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    /// Returns the price of this order in the given denomination.
    ///
    /// The field `price` holds the value in WEI per SATOSHI, hence we need to
    /// multiply it by 100_000_000 to get WEI per BTC.
    pub fn price(&self, denom: Denomination) -> Erc20Quantity {
        let price = self.price.clone();
        match denom {
            Denomination::WeiPerSat => price,
            Denomination::WeiPerBtc => asset::Bitcoin::from_sat(100_000_000) * price,
        }
    }
}

#[cfg(test)]
impl BtcDaiOrder {
    pub fn new_test(
        id: OrderId,
        position: Position,
        quantity: asset::Bitcoin,
        price: Erc20Quantity,
        swap_protocol: SwapProtocol,
        created_at: OffsetDateTime,
    ) -> BtcDaiOrder {
        Self {
            id,
            position,
            quantity,
            price,
            swap_protocol,
            created_at,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Denomination {
    WeiPerBtc,
    WeiPerSat,
}

/// The position of the maker for this order. A BTC/DAI buy order,
/// also described as an order that buys the trading pair BTC/DAI,
/// means that the maker buys the base currency (in this case BTC) in
/// return for DAI. A sell order means that the maker sells BTC and
/// receives DAI.
///
/// Please note: we do not set the base currency to 1 and use rate
/// (i.e., quote currency) and amount as is commonly done in Forex
/// trading. We use the amounts of each currency to determine the
/// rate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, strum_macros::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Position {
    Buy,
    Sell,
}

/// An enum listing all the currently supported swap protocols.
///
/// A swap protocol is a combination of two instances of
/// [`LockProtocol`](crate::LockProtocol)s along with the relevant parameters.
///
/// Each [`SwapProtocol`](SwapProtocol) has an `expiry_offset` parameter that
/// specifies when the lock of the given protocol will expiry based on some
/// reference point in time.
///
/// Given a point X in time, we can do the following calculation:
///
/// - X + hbit_expiry_offset = absolute timestamp for when the
///   [`hbit`](crate::hbit) lock will
/// expire.
/// - X + herc20_expiry_offset = absolute timestamp for when the
/// [`herc20`](crate::herc20) lock will expire.
///
/// Note that an `expiry_offset` is different from the actual locking duration
/// as the two locking protocols start a different points in time.
/// In [`HbitHerc20`](SwapProtocol::HbitHerc20) for example,
/// [`hbit`](crate::hbit) is the [`Alpha`](Side::Alpha) ledger. Since
/// [`Alice`](Role::Alice) always goes first, she will start the
/// [`hbit`](crate::hbit) protocol. [`Bob`](Role::Bob) coming 2nd, will first
/// wait for the lock of the [`hbit`](crate::hbit) protocol to go into effect
/// (waiting for some number of confirmations) before he moves forward with the
/// lock on the [`Beta`](Side::Beta) side. Hence, the _duration_ of the lock
/// differs from the offset specified here.
#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SwapProtocol {
    HbitHerc20 {
        hbit_expiry_offset: Duration,
        herc20_expiry_offset: Duration,
    },
    Herc20Hbit {
        hbit_expiry_offset: Duration,
        herc20_expiry_offset: Duration,
    },
}

impl SwapProtocol {
    pub fn role(&self, position: Position) -> Role {
        match position {
            Position::Buy => match self {
                SwapProtocol::HbitHerc20 { .. } => Role::Bob,
                SwapProtocol::Herc20Hbit { .. } => Role::Alice,
            },
            Position::Sell => match self {
                SwapProtocol::HbitHerc20 { .. } => Role::Alice,
                SwapProtocol::Herc20Hbit { .. } => Role::Bob,
            },
        }
    }

    pub fn position(&self, role: Role) -> Position {
        match role {
            Role::Alice => match self {
                SwapProtocol::HbitHerc20 { .. } => Position::Sell,
                SwapProtocol::Herc20Hbit { .. } => Position::Buy,
            },
            Role::Bob => match self {
                SwapProtocol::HbitHerc20 { .. } => Position::Buy,
                SwapProtocol::Herc20Hbit { .. } => Position::Sell,
            },
        }
    }

    pub fn hbit_expiry_offset(&self) -> Duration {
        match self {
            SwapProtocol::Herc20Hbit {
                hbit_expiry_offset, ..
            } => *hbit_expiry_offset,
            SwapProtocol::HbitHerc20 {
                hbit_expiry_offset, ..
            } => *hbit_expiry_offset,
        }
    }

    pub fn herc20_expiry_offset(&self) -> Duration {
        match self {
            SwapProtocol::Herc20Hbit {
                herc20_expiry_offset,
                ..
            } => *herc20_expiry_offset,
            SwapProtocol::HbitHerc20 {
                herc20_expiry_offset,
                ..
            } => *herc20_expiry_offset,
        }
    }

    pub fn new(
        role: Role,
        position: Position,
        hbit_expiry_offset: Duration,
        herc20_expiry_offset: Duration,
    ) -> Self {
        match position {
            Position::Buy => match role {
                Role::Bob => SwapProtocol::HbitHerc20 {
                    hbit_expiry_offset,
                    herc20_expiry_offset,
                },
                Role::Alice => SwapProtocol::Herc20Hbit {
                    herc20_expiry_offset,
                    hbit_expiry_offset,
                },
            },
            Position::Sell => match role {
                Role::Bob => SwapProtocol::Herc20Hbit {
                    herc20_expiry_offset,
                    hbit_expiry_offset,
                },
                Role::Alice => SwapProtocol::HbitHerc20 {
                    hbit_expiry_offset,
                    herc20_expiry_offset,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proptest;

    proptest::proptest! {
        #[test]
        fn swap_protocol_and_position_interplay(swap_protocol in proptest::order::swap_protocol(), position in proptest::order::position()) {
            let role = swap_protocol.role(position);

            let computed_protocol = SwapProtocol::new(role, position, swap_protocol.hbit_expiry_offset(), swap_protocol.herc20_expiry_offset());
            let computed_position = swap_protocol.position(role);

            assert_eq!(computed_protocol, swap_protocol);
            assert_eq!(computed_position, position);
        }
    }

    proptest::proptest! {
        #[test]
        fn swap_protocol_and_role_interplay(swap_protocol in proptest::order::swap_protocol(), role in proptest::role()) {
            let position = swap_protocol.position(role);

            let computed_protocol = SwapProtocol::new(role, position, swap_protocol.hbit_expiry_offset(), swap_protocol.herc20_expiry_offset());
            let computed_role = swap_protocol.role(position);

            assert_eq!(computed_protocol, swap_protocol);
            assert_eq!(computed_role, role);
        }
    }
}
