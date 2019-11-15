macro_rules! _match_role {
    ($role:ident, $fn:expr) => {{
        use crate::swap_protocols::{
            rfc003::{alice, bob},
            Role,
        };

        #[allow(clippy::redundant_closure_call)]
        match $role {
            Role::Alice => {
                #[allow(dead_code)]
                type ROLE = alice::State<AL, BL, AA, BA>;
                $fn
            }
            Role::Bob => {
                #[allow(dead_code)]
                type ROLE = bob::State<AL, BL, AA, BA>;
                $fn
            }
        }
    }};
}

#[macro_export]
macro_rules! with_swap_types {
    ($swap_types:expr, $fn:expr) => {{
        use crate::{
            db::{AssetKind, LedgerKind, SwapTypes},
            swap_protocols::ledger::{Bitcoin, Ethereum},
        };
        use bitcoin::Amount;
        use ethereum_support::{Erc20Token, EtherQuantity};
        let swap_types: SwapTypes = $swap_types;
        let role = swap_types.role;

        match swap_types {
            SwapTypes {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
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
            SwapTypes {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Erc20,
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
            SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::Bitcoin,
                alpha_asset: AssetKind::Ether,
                beta_asset: AssetKind::Bitcoin,
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
            SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::Bitcoin,
                alpha_asset: AssetKind::Erc20,
                beta_asset: AssetKind::Bitcoin,
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
