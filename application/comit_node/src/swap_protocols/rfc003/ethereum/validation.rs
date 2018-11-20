use ethereum_support::{self, CalculateContractAddress, EtherQuantity, Transaction};
use swap_protocols::{
    ledger::Ethereum,
    rfc003::{
        contains_htlc::{ContainsHtlc, Error},
        ethereum::{EtherHtlc, Htlc},
        state_machine::HtlcParams,
    },
};

impl ContainsHtlc<Ethereum, EtherQuantity> for Transaction {
    fn contains_htlc(
        &self,
        htlc_params: &HtlcParams<Ethereum, EtherQuantity>,
    ) -> Result<ethereum_support::Address, Error<EtherQuantity>> {
        if self.to != None {
            return Err(Error::WrongTransaction);
        }

        if self.input != EtherHtlc::from(htlc_params.clone()).compile_to_hex().into() {
            return Err(Error::WrongTransaction);
        }

        if self.value < htlc_params.asset.wei() {
            return Err(Error::UnexpectedAsset {
                found: EtherQuantity::from_wei(self.value),
                expected: htlc_params.asset,
            });
        }

        Ok(self.from.calculate_contract_address(&self.nonce))
    }
}
