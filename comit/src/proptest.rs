use crate::{order::SwapProtocol, Role};
pub use proptest::prelude::*;

pub mod ethereum {
    use super::*;
    use crate::ethereum::*;

    prop_compose! {
        pub fn address()(bytes in any::<[u8; 20]>()) -> Address {
            bytes.into()
        }
    }

    prop_compose! {
        pub fn hash()(bytes in any::<[u8; 32]>()) -> Hash {
            bytes.into()
        }
    }
}

pub mod asset {
    use super::*;
    use crate::asset::{ethereum::FromWei, *};

    prop_compose! {
        pub fn bitcoin()(sats in any::<u64>()) -> asset::Bitcoin {
            asset::Bitcoin::from_sat(sats)
        }
    }

    prop_compose! {
        pub fn erc20_quantity()(wei in any::<[u64; 4]>()) -> asset::Erc20Quantity {
            asset::Erc20Quantity::from_wei(primitive_types::U256(wei))
        }
    }
}

pub mod order {
    use super::*;
    use crate::{BtcDaiOrder, OrderId, Position, Price, Quantity};

    pub fn position() -> impl Strategy<Value = Position> {
        prop_oneof![Just(Position::Buy), Just(Position::Sell)]
    }

    prop_compose! {
        pub fn order_id()(bytes in any::<[u8; 16]>()) -> OrderId {
            uuid::Builder::from_bytes(bytes)
                .set_variant(uuid::Variant::RFC4122)
                .set_version(uuid::Version::Random)
                .build()
                .into()
        }
    }

    prop_compose! {
        pub fn btc_dai_order()(id in order_id(), price in asset::erc20_quantity(), quantity in asset::bitcoin(), swap_protocol in swap_protocol(), position in position(), created_at in time::offset_date_time()) -> BtcDaiOrder {
            BtcDaiOrder::new_test(
                id,
                position,
                Quantity::new(quantity),
                Price::from_wei_per_sat(price),
                swap_protocol,
                created_at
            )
        }
    }

    pub fn swap_protocol() -> impl Strategy<Value = SwapProtocol> {
        prop_oneof![swap_protocol_hbit_herc20(), swap_protocol_herc20_hbit()]
    }

    prop_compose! {
        pub fn swap_protocol_hbit_herc20()(hbit_expiry_offset in expiries::alpha_offset(), herc20_expiry_offset in expiries::beta_offset()) -> SwapProtocol {
            SwapProtocol::HbitHerc20 {
                hbit_expiry_offset,
                herc20_expiry_offset,
            }
        }
    }

    prop_compose! {
        pub fn swap_protocol_herc20_hbit()(herc20_expiry_offset in expiries::alpha_offset(), hbit_expiry_offset in expiries::beta_offset()) -> SwapProtocol {
            SwapProtocol::Herc20Hbit {
                herc20_expiry_offset,
                hbit_expiry_offset,
            }
        }
    }
}

pub fn role() -> impl Strategy<Value = Role> {
    prop_oneof![Just(Role::Alice), Just(Role::Bob)]
}

pub mod time {
    use super::*;
    use ::time::*;

    prop_compose! {
        pub fn duration()(seconds in any::<i64>()) -> Duration {
            Duration::seconds(seconds)
        }
    }

    prop_compose! {
        pub fn offset_date_time()(unix_timestamp in any::<i32>()) -> OffsetDateTime {
            OffsetDateTime::from_unix_timestamp(unix_timestamp as i64)
        }
    }
}

pub mod expiries {
    use super::*;
    use crate::expiries::{AlphaOffset, BetaOffset};

    prop_compose! {
        pub fn alpha_offset()(duration in time::duration()) -> AlphaOffset {
            duration.into()
        }
    }

    prop_compose! {
        pub fn beta_offset()(duration in time::duration()) -> BetaOffset {
            duration.into()
        }
    }
}
