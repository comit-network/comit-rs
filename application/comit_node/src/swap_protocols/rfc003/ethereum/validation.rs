use ethereum_support::{self, CalculateContractAddress, EtherQuantity};
use swap_protocols::{
    ledger::Ethereum,
    rfc003::{
        ethereum::{EtherHtlc, Htlc},
        state_machine::HtlcParams,
        validation::{Error, IsContainedInTransaction},
    },
};

impl IsContainedInTransaction<Ethereum> for EtherQuantity {
    fn is_contained_in_transaction(
        htlc_params: &HtlcParams<Ethereum, EtherQuantity>,
        tx: ethereum_support::Transaction,
    ) -> Result<ethereum_support::Address, Error<Self>> {
        if tx.to != None {
            return Err(Error::WrongTransaction);
        }

        if tx.input != EtherHtlc::from(htlc_params.clone()).compile_to_hex().into() {
            return Err(Error::WrongTransaction);
        }

        if tx.value < htlc_params.asset.wei() {
            return Err(Error::UnexpectedAsset {
                found: EtherQuantity::from_wei(tx.value),
                expected: htlc_params.asset,
            });
        }

        let from_address: ethereum_support::Address = tx.from;

        Ok(from_address.calculate_contract_address(&tx.nonce))
    }
}
