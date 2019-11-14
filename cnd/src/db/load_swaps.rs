use crate::{
    db::Sqlite,
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            messages::{Accept, Request},
            Ledger,
        },
        SwapId,
    },
};

pub type AcceptedSwap<AL, BL, AA, BA> = (Request<AL, BL, AA, BA>, Accept<AL, BL>);

pub trait LoadAcceptedSwap<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    fn load_accepted_swap(&self, swap_id: SwapId) -> anyhow::Result<AcceptedSwap<AL, BL, AA, BA>>;
}

impl LoadAcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::EtherQuantity>
    for Sqlite
{
    fn load_accepted_swap(
        &self,
        _key: SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::EtherQuantity>,
    > {
        unimplemented!()
    }
}

impl LoadAcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::Erc20Token> for Sqlite {
    fn load_accepted_swap(
        &self,
        _key: SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::Erc20Token>,
    > {
        unimplemented!()
    }
}

impl LoadAcceptedSwap<Ethereum, Bitcoin, ethereum_support::EtherQuantity, bitcoin::Amount>
    for Sqlite
{
    fn load_accepted_swap(
        &self,
        _key: SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<Ethereum, Bitcoin, ethereum_support::EtherQuantity, bitcoin::Amount>,
    > {
        unimplemented!()
    }
}

impl LoadAcceptedSwap<Ethereum, Bitcoin, ethereum_support::Erc20Token, bitcoin::Amount> for Sqlite {
    fn load_accepted_swap(
        &self,
        _key: SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<Ethereum, Bitcoin, ethereum_support::Erc20Token, bitcoin::Amount>,
    > {
        unimplemented!()
    }
}
