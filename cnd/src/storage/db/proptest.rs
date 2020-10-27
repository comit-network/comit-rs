use crate::{proptest::*, storage};

pub mod tables {
    use super::*;
    use comit::Side;
    use storage::db::tables;

    prop_compose! {
        pub fn insertable_swap()(
            local_swap_id in local_swap_id(),
            role in role(),
            peer in libp2p::peer_id(),
            start_of_swap in timestamp(),
        ) -> tables::InsertableSwap {
            tables::InsertableSwap::new(local_swap_id, peer, role, start_of_swap)
        }
    }

    prop_compose! {
        pub fn insertable_hbit(swap_id: i32, side: Side)(
            bitcoin in asset::bitcoin(),
            network in ledger::bitcoin(),
            expiry in any::<u32>(),
            final_identity in bitcoin::address(),
            transient_identity in identity::bitcoin(),
        ) -> tables::InsertableHbit {
            tables::InsertableHbit::new(swap_id, bitcoin, network, expiry, final_identity, transient_identity, side)
        }
    }

    prop_compose! {
        pub fn insertable_herc20(swap_id: i32, side: Side)(
            asset in asset::erc20(),
            ethereum in ledger::ethereum(),
            expiry in any::<u32>(),
            redeem_identity in identity::ethereum(),
            refund_identity in identity::ethereum(),
        ) -> tables::InsertableHerc20 {
            tables::InsertableHerc20::new(swap_id, asset, ethereum.chain_id, expiry, redeem_identity, refund_identity, side)
        }
    }

    prop_compose! {
        pub fn insertable_completed_swap(swap_id: i32)(
            completed_at in timestamp(),
        ) -> tables::InsertableCompletedSwap {
            tables::InsertableCompletedSwap::new(swap_id, completed_at)
        }
    }
}
