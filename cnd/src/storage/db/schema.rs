// LocalSwapId and SharedSwapId are encoded as Text and named local_swap_id, and
// shared_swap_id respectively.  swap_id (Integer) is always a foreign key link
// to the `swaps` table.
table! {
   swaps {
       id -> Integer,
       local_swap_id -> Text,
       role -> Text,
       counterparty_peer_id -> Text,
       start_of_swap -> Timestamp,
   }
}

table! {
   secret_hashes {
       id -> Integer,
       swap_id -> Integer,
       secret_hash -> Text,
   }
}

table! {
    herc20s {
        id -> Integer,
        swap_id -> Integer,
        amount -> Text,
        chain_id -> BigInt,
        expiry -> BigInt,
        token_contract -> Text,
        redeem_identity -> Nullable<Text>,
        refund_identity -> Nullable<Text>,
        side -> Text,
    }
}

table! {
    halbits {
        id -> Integer,
        swap_id -> Integer,
        amount -> Text,
        network -> Text,
        chain -> Text,
        cltv_expiry -> BigInt,
        redeem_identity -> Nullable<Text>,
        refund_identity -> Nullable<Text>,
        side -> Text,
    }
}

table! {
    hbits {
        id -> Integer,
        swap_id -> Integer,
        amount -> Text,
        network -> Text,
        expiry -> BigInt,
        final_identity -> Text,
        transient_identity -> Nullable<Text>,
        side -> Text,
    }
}

table! {
    swap_contexts {
        id -> Integer,
        local_swap_id -> Text,
        role -> Text,
        alpha -> Text,
        beta -> Text,
    }
}

table! {
    orders {
        id -> Integer,
        order_id -> Text,
        position -> Text,
        created_at -> BigInt,
    }
}

table! {
    btc_dai_orders {
        id -> Integer,
        order_id -> Integer,
        quantity -> Text,
        price -> Text,
        open -> Text,
        closed -> Text,
        settling -> Text,
        failed -> Text,
        cancelled -> Text,
    }
}

table! {
    order_hbit_params {
        id -> Integer,
        order_id -> Integer,
        network -> Text,
        side -> Text,
        our_final_address -> Text,
        expiry_offset -> BigInt,
    }
}

table! {
    order_herc20_params {
        id -> Integer,
        order_id -> Integer,
        chain_id -> BigInt,
        side -> Text,
        our_htlc_identity -> Text,
        token_contract -> Text,
        expiry_offset -> BigInt,
    }
}

table! {
    order_swaps {
        id -> Integer,
        order_id -> Integer,
        swap_id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(swaps, halbits);
allow_tables_to_appear_in_same_query!(swaps, herc20s);
allow_tables_to_appear_in_same_query!(swaps, hbits);
allow_tables_to_appear_in_same_query!(halbits, herc20s);
allow_tables_to_appear_in_same_query!(hbits, herc20s);
allow_tables_to_appear_in_same_query!(orders, btc_dai_orders);
joinable!(btc_dai_orders -> orders (order_id));
