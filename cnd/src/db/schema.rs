// LocalSwapId and SharedSwapId are encoded as Text and named local_swap_id, and
// shared_swap_id respectively.  swap_id (Integer) is always a foreign key link
// to the `swaps` table.
table! {
   swaps {
       id -> Integer,
       local_swap_id -> Text,
       role -> Text,
       counterparty_peer_id -> Text,
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
    address_book {
        id -> Integer,
        peer_id -> Text,
        multi_address -> Text,
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
    halights {
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
        redeem_identity -> Nullable<Text>,
        refund_identity -> Nullable<Text>,
        side -> Text,
    }
}

allow_tables_to_appear_in_same_query!(swaps, halights);
allow_tables_to_appear_in_same_query!(swaps, herc20s);
allow_tables_to_appear_in_same_query!(halights, herc20s);
