use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    metadata_store::{AssetKind, LedgerKind, Metadata, RoleKind},
    rfc003::Ledger,
};

#[derive(Clone, Debug, PartialEq, LabelledGeneric)]
pub struct SwapRequest<AL: Ledger, BL: Ledger, AA, BA> {
    pub alpha_asset: AA,
    pub beta_asset: BA,
    pub alpha_ledger: AL,
    pub beta_ledger: BL,
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_success_identity: BL::Identity,
    pub alpha_ledger_lock_duration: AL::LockDuration,
}

impl From<SwapRequest<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>> for Metadata {
    fn from(_: SwapRequest<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>) -> Self {
        Self {
            alpha_ledger: LedgerKind::Bitcoin,
            beta_ledger: LedgerKind::Ethereum,
            alpha_asset: AssetKind::Bitcoin,
            beta_asset: AssetKind::Ether,
            role: RoleKind::Alice,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SwapRequestKind {
    BitcoinEthereumBitcoinQuantityEthereumQuantity(
        SwapRequest<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>,
    ),
}
