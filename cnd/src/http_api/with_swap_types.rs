macro_rules! _match_role {
    ($role:ident, $fn:tt) => {{
        use crate::metadata_store::Role;
        use comit::rfc003::{alice, bob};
        #[allow(clippy::redundant_closure_call)]
        match $role {
            Role::Alice => {
                #[allow(dead_code)]
                type ROLE = alice::State<AL, BL, AA, BA>;
                $fn()
            }
            Role::Bob => {
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
        use crate::metadata_store::{AssetKind, LedgerKind, Metadata};
        use bitcoin_support::Amount;
        use comit::ledger::{Bitcoin, Ethereum};
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
                type AA = Amount;
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
                type AA = Amount;
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
                type BA = Amount;
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
                type BA = Amount;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRedeem<AL>;

                _match_role!(role, $fn)
            }
            _ => unimplemented!(),
        }
    }};
}
