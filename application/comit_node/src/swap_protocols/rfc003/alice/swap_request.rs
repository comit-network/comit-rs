use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    metadata_store::{AssetKind, LedgerKind, Metadata, RoleKind},
    rfc003::Ledger,
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Quantity, EtherQuantity};

#[derive(Clone, Debug, PartialEq)]
pub struct SwapRequest<AL: Ledger, BL: Ledger, AA, BA> {
    pub alpha_asset: AA,
    pub beta_asset: BA,
    pub alpha_ledger: AL,
    pub beta_ledger: BL,
    pub alpha_ledger_lock_duration: AL::LockDuration,
    pub identities: SwapRequestIdentities<AL, BL>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwapRequestIdentities<AL: Ledger, BL: Ledger> {
    pub alpha_ledger_refund_identity: AL::HtlcIdentity,
    pub beta_ledger_redeem_identity: BL::HtlcIdentity,
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

impl From<SwapRequest<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>> for Metadata {
    fn from(_: SwapRequest<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>) -> Self {
        Self {
            alpha_ledger: LedgerKind::Bitcoin,
            beta_ledger: LedgerKind::Ethereum,
            alpha_asset: AssetKind::Bitcoin,
            beta_asset: AssetKind::Erc20,
            role: RoleKind::Alice,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SwapRequestKind {
    BitcoinEthereumBitcoinQuantityEtherQuantity(
        SwapRequest<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>,
    ),
    BitcoinEthereumBitcoinQuantityErc20Quantity(
        SwapRequest<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>,
    ),
}
