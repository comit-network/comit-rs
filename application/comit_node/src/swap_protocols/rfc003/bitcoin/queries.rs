use crate::{
    ledger_query_service::BitcoinQuery,
    swap_protocols::{
        ledger::Bitcoin,
        rfc003::{
            events::{NewHtlcFundedQuery, NewHtlcRedeemedQuery, NewHtlcRefundedQuery},
            state_machine::HtlcParams,
        },
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};

impl NewHtlcFundedQuery<Bitcoin, BitcoinQuantity> for BitcoinQuery {
    fn new_htlc_funded_query(htlc_params: &HtlcParams<Bitcoin, BitcoinQuantity>) -> Self {
        BitcoinQuery::Transaction {
            to_address: Some(htlc_params.compute_address()),
            from_outpoint: None,
            unlock_script: None,
        }
    }
}

impl NewHtlcRefundedQuery<Bitcoin, BitcoinQuantity> for BitcoinQuery {
    fn new_htlc_refunded_query(
        _htlc_params: &HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: &OutPoint,
    ) -> Self {
        BitcoinQuery::Transaction {
            to_address: None,
            from_outpoint: Some(*htlc_location),
            unlock_script: Some(vec![vec![0u8]]),
        }
    }
}

impl NewHtlcRedeemedQuery<Bitcoin, BitcoinQuantity> for BitcoinQuery {
    fn new_htlc_redeemed_query(
        _htlc_params: &HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: &OutPoint,
    ) -> Self {
        BitcoinQuery::Transaction {
            to_address: None,
            from_outpoint: Some(*htlc_location),
            unlock_script: Some(vec![vec![1u8]]),
        }
    }
}
