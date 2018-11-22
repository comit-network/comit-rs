use ethereum_support::{web3::types::Address, Bytes, Erc20Quantity, EtherQuantity};
use ledger_query_service::EthereumQuery;
use swap_protocols::{
    ledger::Ethereum,
    rfc003::{
        events::{NewHtlcFundedQuery, NewHtlcRedeemedQuery, NewHtlcRefundedQuery},
        secret,
        state_machine::HtlcParams,
    },
};

impl NewHtlcFundedQuery<Ethereum, EtherQuantity> for EthereumQuery {
    fn new_htlc_funded_query(htlc_params: &HtlcParams<Ethereum, EtherQuantity>) -> Self {
        EthereumQuery::Transaction {
            from_address: None,
            to_address: None,
            is_contract_creation: Some(true),
            transaction_data: Some(htlc_params.bytecode()),
            transaction_data_length: None,
        }
    }
}

impl NewHtlcRefundedQuery<Ethereum, EtherQuantity> for EthereumQuery {
    fn new_htlc_refunded_query(
        _htlc_params: &HtlcParams<Ethereum, EtherQuantity>,
        htlc_location: &Address,
    ) -> Self {
        EthereumQuery::Transaction {
            from_address: None,
            to_address: Some(htlc_location.clone()),
            is_contract_creation: Some(false),
            transaction_data: Some(Bytes::from(vec![])),
            transaction_data_length: None,
        }
    }
}

impl NewHtlcRedeemedQuery<Ethereum, EtherQuantity> for EthereumQuery {
    fn new_htlc_redeemed_query(
        _htlc_params: &HtlcParams<Ethereum, EtherQuantity>,
        htlc_location: &Address,
    ) -> Self {
        EthereumQuery::Transaction {
            from_address: None,
            to_address: Some(htlc_location.clone()),
            is_contract_creation: Some(false),
            transaction_data: None,
            transaction_data_length: Some(secret::SECRET_LENGTH),
        }
    }
}

impl NewHtlcFundedQuery<Ethereum, Erc20Quantity> for EthereumQuery {
    fn new_htlc_funded_query(htlc_params: &HtlcParams<Ethereum, Erc20Quantity>) -> Self {
        EthereumQuery::Transaction {
            from_address: None,
            to_address: None,
            is_contract_creation: Some(true),
            transaction_data: Some(htlc_params.bytecode()),
            transaction_data_length: None,
        }
    }
}

impl NewHtlcRefundedQuery<Ethereum, Erc20Quantity> for EthereumQuery {
    fn new_htlc_refunded_query(
        _htlc_params: &HtlcParams<Ethereum, Erc20Quantity>,
        htlc_location: &Address,
    ) -> Self {
        EthereumQuery::Transaction {
            from_address: None,
            to_address: Some(htlc_location.clone()),
            is_contract_creation: Some(false),
            transaction_data: Some(Bytes::from(vec![])),
            transaction_data_length: None,
        }
    }
}

impl NewHtlcRedeemedQuery<Ethereum, Erc20Quantity> for EthereumQuery {
    fn new_htlc_redeemed_query(
        _htlc_params: &HtlcParams<Ethereum, Erc20Quantity>,
        htlc_location: &Address,
    ) -> Self {
        EthereumQuery::Transaction {
            from_address: None,
            to_address: Some(htlc_location.clone()),
            is_contract_creation: Some(false),
            transaction_data: None,
            transaction_data_length: Some(secret::SECRET_LENGTH),
        }
    }
}
