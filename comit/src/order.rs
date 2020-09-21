use crate::{
    asset::{Bitcoin, Erc20Quantity},
    expiries,
    expiries::{AlphaOffset, BetaOffset},
    Network, Role,
};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, marker::PhantomData, str::FromStr};
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BtcDaiOrder {
    pub id: OrderId,
    pub position: Position,
    pub swap_protocol: SwapProtocol,
    pub created_at: OffsetDateTime,
    pub quantity: Quantity<Bitcoin>,
    pub price: Price<Bitcoin, Erc20Quantity>,
}

impl BtcDaiOrder {
    pub fn buy(
        quantity: Quantity<Bitcoin>,
        price: Price<Bitcoin, Erc20Quantity>,
        swap_protocol: SwapProtocol,
    ) -> Self {
        Self::new(Position::Buy, quantity, price, swap_protocol)
    }

    pub fn sell(
        quantity: Quantity<Bitcoin>,
        price: Price<Bitcoin, Erc20Quantity>,
        swap_protocol: SwapProtocol,
    ) -> Self {
        Self::new(Position::Sell, quantity, price, swap_protocol)
    }

    pub fn new(
        position: Position,
        quantity: Quantity<Bitcoin>,
        price: Price<Bitcoin, Erc20Quantity>,
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
}

/// A newtype representing a quantity in a certain base currency B.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Quantity<B> {
    inner: B,
}

impl Quantity<Bitcoin> {
    pub fn new(value: Bitcoin) -> Self {
        Self { inner: value }
    }

    pub fn sats(&self) -> u64 {
        self.inner.as_sat()
    }

    /// The [`Bitcoin`] type encapsulates sats and btc well, hence we can just
    /// provide access to the inner value here.
    pub fn to_inner(&self) -> Bitcoin {
        self.inner
    }
}

#[cfg(test)]
pub fn btc(btc: f64) -> Quantity<Bitcoin> {
    Quantity::new(Bitcoin::from_sat(
        bitcoin::Amount::from_btc(btc).unwrap().as_sat(),
    ))
}

/// A newtype representing the price of one unit of the base currency B in the
/// quote currency Q.
///
/// The core idea around of this type is to enforce the unit of the rate when
/// doing calculations. We achieve that by adding "loud" constructors and
/// accessors for combinations of base and quote currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Price<B, Q> {
    inner: Q,
    _base: PhantomData<B>,
}

impl Price<Bitcoin, Erc20Quantity> {
    /// Constructs a new instance of Price where the rate is given in WEI/SAT.
    ///
    /// This is how we store the data internally and hence we don't need to do
    /// any conversions.
    pub fn from_wei_per_sat(rate: Erc20Quantity) -> Self {
        Price {
            inner: rate,
            _base: PhantomData,
        }
    }

    pub fn wei_per_sat(&self) -> Erc20Quantity {
        self.inner.clone()
    }

    pub fn wei_per_btc(&self) -> Erc20Quantity {
        self.inner
            .clone()
            .checked_mul(100_000_000)
            .expect("the price of bitcoin to not go through the roof")
    }
}

#[cfg(test)]
pub fn dai_per_btc(dai: u64) -> Price<Bitcoin, Erc20Quantity> {
    use crate::asset::ethereum::TryFromWei;

    let dai_precision = 18u32;
    let btc_precision = 8;

    let factor = num::BigUint::from(10u32).pow(dai_precision - btc_precision);
    let rate = Erc20Quantity::try_from_wei(dai * factor).unwrap();

    Price::from_wei_per_sat(rate)
}

#[cfg(test)]
impl BtcDaiOrder {
    pub fn new_test(
        id: OrderId,
        position: Position,
        quantity: Quantity<Bitcoin>,
        price: Price<Bitcoin, Erc20Quantity>,
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
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    strum_macros::EnumString,
    strum_macros::Display,
    strum_macros::EnumIter,
)]
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SwapProtocol {
    HbitHerc20 {
        hbit_expiry_offset: AlphaOffset,
        herc20_expiry_offset: BetaOffset,
    },
    Herc20Hbit {
        herc20_expiry_offset: AlphaOffset,
        hbit_expiry_offset: BetaOffset,
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
            } => Duration::from(*hbit_expiry_offset),
            SwapProtocol::HbitHerc20 {
                hbit_expiry_offset, ..
            } => Duration::from(*hbit_expiry_offset),
        }
    }

    pub fn herc20_expiry_offset(&self) -> Duration {
        match self {
            SwapProtocol::Herc20Hbit {
                herc20_expiry_offset,
                ..
            } => Duration::from(*herc20_expiry_offset),
            SwapProtocol::HbitHerc20 {
                herc20_expiry_offset,
                ..
            } => Duration::from(*herc20_expiry_offset),
        }
    }

    pub fn new(role: Role, position: Position, network: Network) -> Self {
        match (role, position) {
            (Role::Bob, Position::Buy) | (Role::Alice, Position::Sell) => {
                let (hbit_expiry_offset, herc20_expiry_offset) =
                    expiries::expiry_offsets_hbit_herc20(network);

                SwapProtocol::HbitHerc20 {
                    hbit_expiry_offset,
                    herc20_expiry_offset,
                }
            }
            (Role::Alice, Position::Buy) | (Role::Bob, Position::Sell) => {
                let (herc20_expiry_offset, hbit_expiry_offset) =
                    expiries::expiry_offsets_herc20_hbit(network);

                SwapProtocol::Herc20Hbit {
                    hbit_expiry_offset,
                    herc20_expiry_offset,
                }
            }
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

            let computed_position = SwapProtocol::new(role, position, Network::Main).position(role);

            assert_eq!(computed_position, position);
        }
    }

    proptest::proptest! {
        #[test]
        fn swap_protocol_and_role_interplay(swap_protocol in proptest::order::swap_protocol(), role in proptest::role()) {
            let position = swap_protocol.position(role);

            let computed_role = SwapProtocol::new(role, position, Network::Main).role(position);

            assert_eq!(computed_role, role);
        }
    }

    #[test]
    fn dai_per_btc_turns_into_wei_per_sat() {
        // 1 BTC : 9_000 DAI = 1 BTC : 9_000_000_000_000_000_000_000 WEI = 100_000_000
        // SAT : 9_000_000_000_000_000_000_000 WEI = 1 SAT : 90_000_000_000_000 WEI
        let wei_per_sat = Erc20Quantity::from_wei_dec_str("90000000000000").unwrap();

        assert_eq!(dai_per_btc(9000), Price::from_wei_per_sat(wei_per_sat))
    }
}
