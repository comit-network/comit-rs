use ethereum_support::{web3::types::Address, EtherQuantity};
use ledger_query_service::EthereumQuery;
use swap_protocols::{
    asset::Asset,
    ledger::Ethereum,
    rfc003::{
        events::{NewTargetHtlcRedeemedQuery, NewTargetHtlcRefundedQuery},
        state_machine::OngoingSwap,
        IntoSecretHash, Ledger,
    },
};

impl<SL, SA, S> NewTargetHtlcRefundedQuery<SL, Ethereum, SA, EtherQuantity, S> for EthereumQuery
where
    SL: Ledger,
    SA: Asset,
    S: IntoSecretHash,
{
    fn new_target_htlc_refunded_query(
        _swap: &OngoingSwap<SL, Ethereum, SA, EtherQuantity, S>,
        target_htlc_location: &Address,
    ) -> Self {
        EthereumQuery::Transaction {
            from_address: None,
            to_address: Some(target_htlc_location.clone()),
            is_contract_creation: Some(false),
            transaction_data: None,
        }
    }
}

impl<SL, SA, S> NewTargetHtlcRedeemedQuery<SL, Ethereum, SA, EtherQuantity, S> for EthereumQuery
where
    SL: Ledger,
    SA: Asset,
    S: IntoSecretHash,
{
    fn new_target_htlc_redeemed_query(
        _swap: &OngoingSwap<SL, Ethereum, SA, EtherQuantity, S>,
        target_htlc_location: &Address,
    ) -> Self {
        EthereumQuery::Transaction {
            from_address: None,
            to_address: Some(target_htlc_location.clone()),
            is_contract_creation: Some(false),
            transaction_data: None,
        }
    }
}
