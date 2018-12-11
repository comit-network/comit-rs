use crate::swap_protocols::{
    ledger::Ethereum,
    rfc003::{
        ethereum::{Erc20Htlc, EtherHtlc, Htlc},
        find_htlc_location::{compare_assets, Error, FindHtlcLocation},
        state_machine::HtlcParams,
    },
};
use ethereum_support::{self, CalculateContractAddress, Erc20Quantity, EtherQuantity, Transaction};

impl FindHtlcLocation<Ethereum, EtherQuantity> for Transaction {
    fn find_htlc_location(
        &self,
        htlc_params: &HtlcParams<Ethereum, EtherQuantity>,
    ) -> Result<ethereum_support::Address, Error<EtherQuantity>> {
        if self.to != None {
            return Err(Error::WrongTransaction);
        }

        if self.input != EtherHtlc::from(htlc_params.clone()).compile_to_hex().into() {
            return Err(Error::WrongTransaction);
        }

        let location = self.from.calculate_contract_address(&self.nonce);
        let actual_value = EtherQuantity::from_wei(self.value);
        let required_value = htlc_params.asset;

        compare_assets(location, actual_value, required_value)
    }
}

impl FindHtlcLocation<Ethereum, Erc20Quantity> for Transaction {
    fn find_htlc_location(
        &self,
        htlc_params: &HtlcParams<Ethereum, Erc20Quantity>,
    ) -> Result<ethereum_support::Address, Error<Erc20Quantity>> {
        if self.to != None {
            return Err(Error::WrongTransaction);
        }

        if self.input != Erc20Htlc::from(htlc_params.clone()).compile_to_hex().into() {
            return Err(Error::WrongTransaction);
        }

        Ok(self.from.calculate_contract_address(&self.nonce))
    }
}
