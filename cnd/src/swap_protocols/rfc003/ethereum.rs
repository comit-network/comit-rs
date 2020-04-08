pub mod htlc_events;

use crate::{
    asset,
    ethereum::Bytes,
    identity,
    swap_protocols::{ledger::Ethereum, rfc003::create_swap::HtlcParams},
};
use blockchain_contracts::ethereum::rfc003::{erc20_htlc::Erc20Htlc, ether_htlc::EtherHtlc};

impl From<HtlcParams<Ethereum, asset::Ether, identity::Ethereum>> for EtherHtlc {
    fn from(htlc_params: HtlcParams<Ethereum, asset::Ether, identity::Ethereum>) -> Self {
        let refund_address =
            blockchain_contracts::ethereum::Address(htlc_params.refund_identity.into());
        let redeem_address =
            blockchain_contracts::ethereum::Address(htlc_params.redeem_identity.into());

        EtherHtlc::new(
            htlc_params.expiry.into(),
            refund_address,
            redeem_address,
            htlc_params.secret_hash.into(),
        )
    }
}

impl HtlcParams<Ethereum, asset::Ether, identity::Ethereum> {
    pub fn bytecode(&self) -> Bytes {
        EtherHtlc::from(self.clone()).into()
    }
}

impl From<HtlcParams<Ethereum, asset::Erc20, identity::Ethereum>> for Erc20Htlc {
    fn from(htlc_params: HtlcParams<Ethereum, asset::Erc20, identity::Ethereum>) -> Self {
        let refund_address =
            blockchain_contracts::ethereum::Address(htlc_params.refund_identity.into());
        let redeem_address =
            blockchain_contracts::ethereum::Address(htlc_params.redeem_identity.into());
        let token_contract_address =
            blockchain_contracts::ethereum::Address(htlc_params.asset.token_contract.into());

        Erc20Htlc::new(
            htlc_params.expiry.into(),
            refund_address,
            redeem_address,
            htlc_params.secret_hash.into(),
            token_contract_address,
            htlc_params.asset.quantity.into(),
        )
    }
}

impl HtlcParams<Ethereum, asset::Erc20, identity::Ethereum> {
    pub fn bytecode(&self) -> Bytes {
        Erc20Htlc::from(self.clone()).into()
    }
}
