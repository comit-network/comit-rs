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
   shared_swap_ids {
       id -> Integer,
       swap_id -> Integer,
       shared_swap_id -> Text,
   }
}

table! {
    address_hints {
        id -> Integer,
        peer_id -> Text,
        address_hint -> Text,
    }
}

table! {
    herc20s {
        id -> Integer,
        swap_id -> Integer,
        amount -> Text,
        chain_id -> BigInt,
        expiry -> BigInt,
        hash_function -> Text,
        token_contract -> Text,
        redeem_identity -> Nullable<Text>,
        refund_identity -> Nullable<Text>,
        ledger -> Text,
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
        hash_function -> Text,
        redeem_identity -> Nullable<Text>,
        refund_identity -> Nullable<Text>,
        ledger -> Text,
    }
}
