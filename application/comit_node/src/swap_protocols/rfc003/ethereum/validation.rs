use swap_protocols::ledger::Ethereum;

use ethereum_support::{self, EtherQuantity};
use swap_protocols::rfc003::{
    state_machine::HtlcParams,
    validation::{Error, IsContainedInTransaction},
};

impl IsContainedInTransaction<Ethereum> for EtherQuantity {
    fn is_contained_in_transaction(
        _htlc_params: &HtlcParams<Ethereum, EtherQuantity>,
        _transaction: ethereum_support::Transaction,
    ) -> Result<ethereum_support::Address, Error<Self>> {
        unimplemented!()
    }
}
