use bitcoin_support::{BitcoinQuantity, OutPoint};
use ledger_query_service::BitcoinQuery;
use swap_protocols::{
    asset::Asset,
    ledger::Bitcoin,
    rfc003::{
        bitcoin::bitcoin_htlc_address,
        events::{
            NewSourceHtlcFundedQuery, NewSourceHtlcRedeemedQuery, NewSourceHtlcRefundedQuery,
        },
        state_machine::OngoingSwap,
       IntoSecretHash, Ledger,
    },
};

impl<TL, TA, S> NewSourceHtlcFundedQuery<Bitcoin, TL, BitcoinQuantity, TA, S> for BitcoinQuery
where
    TL: Ledger,
    TA: Asset,
    S: IntoSecretHash,
{
    fn new_source_htlc_funded_query(
        swap: &OngoingSwap<Bitcoin, TL, BitcoinQuantity, TA, S>,
    ) -> Self {
        BitcoinQuery::Transaction {
            to_address: Some(bitcoin_htlc_address(swap)),
            unlock_script: None,
        }
    }
}

impl<TL, TA, S> NewSourceHtlcRefundedQuery<Bitcoin, TL, BitcoinQuantity, TA, S> for BitcoinQuery
where
    TL: Ledger,
    TA: Asset,
    S: Into<SecretHash> + Clone,
{
    fn new_source_htlc_refunded_query(
        swap: &OngoingSwap<Bitcoin, TL, BitcoinQuantity, TA, S>,
        _source_htlc_location: &OutPoint, //TODO:  this will be used once we have filter by `to_address` implemented
    ) -> Self {
        BitcoinQuery::Transaction {
            to_address: None,
            unlock_script: Some(vec![
                swap.source_identity
                    .public_key()
                    .inner()
                    .serialize()
                    .to_vec(),
                vec![0u8],
            ]),
        }
    }
}

impl<TL, TA, S> NewSourceHtlcRedeemedQuery<Bitcoin, TL, BitcoinQuantity, TA, S> for BitcoinQuery
where
    TL: Ledger,
    TA: Asset,
    S: Into<SecretHash> + Clone,
{
    fn new_source_htlc_redeemed_query(
        swap: &OngoingSwap<Bitcoin, TL, BitcoinQuantity, TA, S>,
        _source_htlc_location: &OutPoint, //TODO:  this will be used once we have filter by `to_address` implemented
    ) -> Self {
        BitcoinQuery::Transaction {
            to_address: None,
            unlock_script: Some(vec![vec![1u8]]),
        }
    }
}
