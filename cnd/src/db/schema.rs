table! {
   rfc003_bitcoin_ethereum_bitcoin_ether_request_messages {
       id -> Integer,
       swap_id -> Text,
       bitcoin_network -> Text,
       ethereum_chain_id -> BigInt,
       bitcoin_amount -> Text,
       ether_amount -> Text,
       hash_function -> Text,
       bitcoin_refund_identity -> Text,
       ethereum_redeem_identity -> Text,
       bitcoin_expiry -> BigInt,
       ethereum_expiry -> BigInt,
       secret_hash -> Text,
   }
}

table! {
   rfc003_ethereum_bitcoin_ether_bitcoin_request_messages {
       id -> Integer,
       swap_id -> Text,
       ethereum_chain_id -> BigInt,
       bitcoin_network -> Text,
       ether_amount -> Text,
       bitcoin_amount -> Text,
       hash_function -> Text,
       ethereum_refund_identity -> Text,
       bitcoin_redeem_identity -> Text,
       ethereum_expiry -> BigInt,
       bitcoin_expiry -> BigInt,
       secret_hash -> Text,
   }
}

table! {
   rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages {
       id -> Integer,
       swap_id -> Text,
       bitcoin_network -> Text,
       ethereum_chain_id -> BigInt,
       bitcoin_amount -> Text,
       erc20_amount -> Text,
       erc20_token_contract -> Text,
       hash_function -> Text,
       bitcoin_refund_identity -> Text,
       ethereum_redeem_identity -> Text,
       bitcoin_expiry -> BigInt,
       ethereum_expiry -> BigInt,
       secret_hash -> Text,
   }
}

table! {
   rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages {
       id -> Integer,
       swap_id -> Text,
       ethereum_chain_id -> BigInt,
       bitcoin_network -> Text,
       erc20_amount -> Text,
       erc20_token_contract -> Text,
       bitcoin_amount -> Text,
       hash_function -> Text,
       ethereum_refund_identity -> Text,
       bitcoin_redeem_identity -> Text,
       ethereum_expiry -> BigInt,
       bitcoin_expiry -> BigInt,
       secret_hash -> Text,
   }
}

table! {
   rfc003_ethereum_bitcoin_accept_messages {
       id -> Integer,
       swap_id -> Text,
       ethereum_redeem_identity -> Text,
       bitcoin_refund_identity -> Text,
       at -> Timestamp,
   }
}

table! {
   rfc003_bitcoin_ethereum_accept_messages {
       id -> Integer,
       swap_id -> Text,
       bitcoin_redeem_identity -> Text,
       ethereum_refund_identity -> Text,
       at -> Timestamp,
   }
}

table! {
   rfc003_decline_messages {
       id -> Integer,
       swap_id -> Text,
       reason -> Nullable<Text>,
   }
}

table! {
   rfc003_swaps {
       id -> Integer,
       swap_id -> Text,
       role -> Text,
       counterparty -> Text,
   }
}

// We need a way to tie the 'created' swaps with the 'finalized' swaps, possible
// methods include:
//
// - Add a `local_id` row to the finalized swaps table then use an SQL join to
//   get the right row.
//
// - Add `finalized_id` to 'created' swaps table that refers to the `id` in
//   `finalized_swaps`.

table! {
    han_ethereum_ether_halight_lightning_bitcoin_created_swaps {
        id -> Integer,
        role -> Text,
        dial_information -> Text,
        ethereum_chain_id -> BigInt,
        bitcoin_network -> Text,
        ether_amount -> Text,
        bitcoin_amount -> Text,
        alpha_identity -> Text,
        beta_identity -> Text,
        secret_hash -> Text,
        ethereum_expiry -> BigInt,
        bitcoin_expiry -> BigInt, // FIXME: Is this correct?
        local_id -> Text,
    }
}
