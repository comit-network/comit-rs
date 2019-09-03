macro_rules! _match_role {
    ($role:ident, $fn:tt) => {{
        use crate::swap_protocols::{
            metadata_store::RoleKind,
            rfc003::{alice, bob},
        };
        #[allow(clippy::redundant_closure_call)]
        match $role {
            RoleKind::Alice => {
                #[allow(dead_code)]
                type ROLE = alice::State<AL, BL, AA, BA>;
                $fn()
            }
            RoleKind::Bob => {
                #[allow(dead_code)]
                type ROLE = bob::State<AL, BL, AA, BA>;
                $fn()
            }
        }
    }};
}

#[macro_export]
macro_rules! with_swap_types {
    ($metadata:expr, $fn:tt) => {{
        use crate::swap_protocols::{
            ledger::{Bitcoin, Ethereum},
            metadata_store::{AssetKind, LedgerKind, Metadata},
        };
        use bitcoin_support::BitcoinQuantity;
        use ethereum_support::{Erc20Token, EtherQuantity};
        let metadata = $metadata;

        match metadata {
            Metadata {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
                role,
                ..
            } => {
                #[allow(dead_code)]
                type AL = Bitcoin;
                #[allow(dead_code)]
                type BL = Ethereum;
                #[allow(dead_code)]
                type AA = BitcoinQuantity;
                #[allow(dead_code)]
                type BA = EtherQuantity;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRefund<BL>;

                _match_role!(role, $fn)
            }
            Metadata {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Erc20,
                role,
                ..
            } => {
                #[allow(dead_code)]
                type AL = Bitcoin;
                #[allow(dead_code)]
                type BL = Ethereum;
                #[allow(dead_code)]
                type AA = BitcoinQuantity;
                #[allow(dead_code)]
                type BA = Erc20Token;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRefund<BL>;

                _match_role!(role, $fn)
            }
            Metadata {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::Bitcoin,
                alpha_asset: AssetKind::Ether,
                beta_asset: AssetKind::Bitcoin,
                role,
                ..
            } => {
                #[allow(dead_code)]
                type AL = Ethereum;
                #[allow(dead_code)]
                type BL = Bitcoin;
                #[allow(dead_code)]
                type AA = EtherQuantity;
                #[allow(dead_code)]
                type BA = BitcoinQuantity;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRedeem<AL>;

                _match_role!(role, $fn)
            }
            Metadata {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::Bitcoin,
                alpha_asset: AssetKind::Erc20,
                beta_asset: AssetKind::Bitcoin,
                role,
                ..
            } => {
                #[allow(dead_code)]
                type AL = Ethereum;
                #[allow(dead_code)]
                type BL = Bitcoin;
                #[allow(dead_code)]
                type AA = Erc20Token;
                #[allow(dead_code)]
                type BA = BitcoinQuantity;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRedeem<AL>;

                _match_role!(role, $fn)
            }
            _ => unimplemented!(),
        }
    }};
}
