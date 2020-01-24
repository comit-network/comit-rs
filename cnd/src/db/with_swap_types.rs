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
            asset,
            db::{AssetKind, LedgerKind, SwapTypes},
            swap_protocols::ledger::{bitcoin, Ethereum},
        };
        let swap_types: SwapTypes = $swap_types;
        let role = swap_types.role;

        match swap_types {
            SwapTypes {
                alpha_ledger: LedgerKind::BitcoinMainnet,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
                ..
            } => {
                #[allow(dead_code)]
                type AL = bitcoin::Mainnet;
                #[allow(dead_code)]
                type BL = Ethereum;
                #[allow(dead_code)]
                type AA = asset::Bitcoin;
                #[allow(dead_code)]
                type BA = asset::Ether;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRefund<BL>;

                _match_role!(role, $fn)
            }
            SwapTypes {
                alpha_ledger: LedgerKind::BitcoinMainnet,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Erc20,
                ..
            } => {
                #[allow(dead_code)]
                type AL = bitcoin::Mainnet;
                #[allow(dead_code)]
                type BL = Ethereum;
                #[allow(dead_code)]
                type AA = asset::Bitcoin;
                #[allow(dead_code)]
                type BA = asset::Erc20;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRefund<BL>;

                _match_role!(role, $fn)
            }
            SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::BitcoinMainnet,
                alpha_asset: AssetKind::Ether,
                beta_asset: AssetKind::Bitcoin,
                ..
            } => {
                #[allow(dead_code)]
                type AL = Ethereum;
                #[allow(dead_code)]
                type BL = bitcoin::Mainnet;
                #[allow(dead_code)]
                type AA = asset::Ether;
                #[allow(dead_code)]
                type BA = asset::Bitcoin;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRedeem<AL>;

                _match_role!(role, $fn)
            }
            SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::BitcoinMainnet,
                alpha_asset: AssetKind::Erc20,
                beta_asset: AssetKind::Bitcoin,
                ..
            } => {
                #[allow(dead_code)]
                type AL = Ethereum;
                #[allow(dead_code)]
                type BL = bitcoin::Mainnet;
                #[allow(dead_code)]
                type AA = asset::Erc20;
                #[allow(dead_code)]
                type BA = asset::Bitcoin;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRedeem<AL>;

                _match_role!(role, $fn)
            }
            SwapTypes {
                alpha_ledger: LedgerKind::BitcoinTestnet,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
                ..
            } => {
                #[allow(dead_code)]
                type AL = bitcoin::Testnet;
                #[allow(dead_code)]
                type BL = Ethereum;
                #[allow(dead_code)]
                type AA = asset::Bitcoin;
                #[allow(dead_code)]
                type BA = asset::Ether;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRefund<BL>;

                _match_role!(role, $fn)
            }
            SwapTypes {
                alpha_ledger: LedgerKind::BitcoinTestnet,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Erc20,
                ..
            } => {
                #[allow(dead_code)]
                type AL = bitcoin::Testnet;
                #[allow(dead_code)]
                type BL = Ethereum;
                #[allow(dead_code)]
                type AA = asset::Bitcoin;
                #[allow(dead_code)]
                type BA = asset::Erc20;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRefund<BL>;

                _match_role!(role, $fn)
            }
            SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::BitcoinTestnet,
                alpha_asset: AssetKind::Ether,
                beta_asset: AssetKind::Bitcoin,
                ..
            } => {
                #[allow(dead_code)]
                type AL = Ethereum;
                #[allow(dead_code)]
                type BL = bitcoin::Testnet;
                #[allow(dead_code)]
                type AA = asset::Ether;
                #[allow(dead_code)]
                type BA = asset::Bitcoin;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRedeem<AL>;

                _match_role!(role, $fn)
            }
            SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::BitcoinTestnet,
                alpha_asset: AssetKind::Erc20,
                beta_asset: AssetKind::Bitcoin,
                ..
            } => {
                #[allow(dead_code)]
                type AL = Ethereum;
                #[allow(dead_code)]
                type BL = bitcoin::Testnet;
                #[allow(dead_code)]
                type AA = asset::Erc20;
                #[allow(dead_code)]
                type BA = asset::Bitcoin;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRedeem<AL>;

                _match_role!(role, $fn)
            }
            SwapTypes {
                alpha_ledger: LedgerKind::BitcoinRegtest,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
                ..
            } => {
                #[allow(dead_code)]
                type AL = bitcoin::Regtest;
                #[allow(dead_code)]
                type BL = Ethereum;
                #[allow(dead_code)]
                type AA = asset::Bitcoin;
                #[allow(dead_code)]
                type BA = asset::Ether;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRefund<BL>;

                _match_role!(role, $fn)
            }
            SwapTypes {
                alpha_ledger: LedgerKind::BitcoinRegtest,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Erc20,
                ..
            } => {
                #[allow(dead_code)]
                type AL = bitcoin::Regtest;
                #[allow(dead_code)]
                type BL = Ethereum;
                #[allow(dead_code)]
                type AA = asset::Bitcoin;
                #[allow(dead_code)]
                type BA = asset::Erc20;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRefund<BL>;

                _match_role!(role, $fn)
            }
            SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::BitcoinRegtest,
                alpha_asset: AssetKind::Ether,
                beta_asset: AssetKind::Bitcoin,
                ..
            } => {
                #[allow(dead_code)]
                type AL = Ethereum;
                #[allow(dead_code)]
                type BL = bitcoin::Regtest;
                #[allow(dead_code)]
                type AA = asset::Ether;
                #[allow(dead_code)]
                type BA = asset::Bitcoin;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRedeem<AL>;

                _match_role!(role, $fn)
            }
            SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::BitcoinRegtest,
                alpha_asset: AssetKind::Erc20,
                beta_asset: AssetKind::Bitcoin,
                ..
            } => {
                #[allow(dead_code)]
                type AL = Ethereum;
                #[allow(dead_code)]
                type BL = bitcoin::Regtest;
                #[allow(dead_code)]
                type AA = asset::Erc20;
                #[allow(dead_code)]
                type BA = asset::Bitcoin;
                #[allow(dead_code)]
                type AcceptBody = crate::http_api::routes::rfc003::accept::OnlyRedeem<AL>;

                _match_role!(role, $fn)
            }
            _ => unimplemented!(),
        }
    }};
}
