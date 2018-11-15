use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    metadata_store::{Assets, Ledgers, Metadata, Roles},
    rfc003::Ledger,
};

#[derive(Clone, Debug, PartialEq, LabelledGeneric)]
pub struct AliceSwapRequest<SL: Ledger, TL: Ledger, SA, TA> {
    pub source_asset: SA,
    pub target_asset: TA,
    pub source_ledger: SL,
    pub target_ledger: TL,
    pub source_ledger_refund_identity: SL::Identity,
    pub target_ledger_success_identity: TL::Identity,
    pub source_ledger_lock_duration: SL::LockDuration,
}

impl From<AliceSwapRequest<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>> for Metadata {
    fn from(_: AliceSwapRequest<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>) -> Self {
        Self {
            source_ledger: Ledgers::Bitcoin,
            target_ledger: Ledgers::Ethereum,
            source_asset: Assets::Bitcoin,
            target_asset: Assets::Ether,
            role: Roles::Alice,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AliceSwapRequests {
    BitcoinEthereumBitcoinQuantityEthereumQuantity(
        AliceSwapRequest<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>,
    ),
}
