use crate::{proptest::*, storage};
use std::fmt::Debug;

pub fn created_swap<A, B>(
    alpha: impl Strategy<Value = A>,
    beta: impl Strategy<Value = B>,
) -> impl Strategy<Value = storage::CreatedSwap<A, B>>
where
    A: Debug,
    B: Debug,
{
    (
        local_swap_id(),
        alpha,
        beta,
        libp2p::peer_id(),
        role(),
        timestamp(),
    )
        .prop_map(
            |(swap_id, alpha, beta, peer, role, start_of_swap)| storage::CreatedSwap {
                swap_id,
                alpha,
                beta,
                peer,
                address_hint: None,
                role,
                start_of_swap,
            },
        )
}

pub mod tables {
    use super::*;
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
}
