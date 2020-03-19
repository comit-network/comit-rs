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
                type RoleState = alice::State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>;
                $fn
            }
            Role::Bob => {
                #[allow(dead_code)]
                type RoleState = bob::State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>;
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
            db::{AssetKind, BitcoinLedgerKind, LedgerKind, SwapTypes},
            htlc_location, identity,
            swap_protocols::ledger::{bitcoin, Ethereum},
            transaction,
        };
        let swap_types: SwapTypes = $swap_types;
        let role = swap_types.role;

        match swap_types {
            SwapTypes {
                alpha_ledger: LedgerKind::Bitcoin(bitcoin_ledger_kind),
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
                ..
            } => match bitcoin_ledger_kind {
                BitcoinLedgerKind::Mainnet => {
                    #[allow(dead_code)]
                    type AL = bitcoin::Mainnet;
                    #[allow(dead_code)]
                    type BL = Ethereum;
                    #[allow(dead_code)]
                    type AA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type BA = asset::Ether;
                    #[allow(dead_code)]
                    type AH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type BH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type AI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type BI = identity::Ethereum;
                    #[allow(dead_code)]
                    type AT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type BT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRefund<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
                BitcoinLedgerKind::Testnet => {
                    #[allow(dead_code)]
                    type AL = bitcoin::Testnet;
                    #[allow(dead_code)]
                    type BL = Ethereum;
                    #[allow(dead_code)]
                    type AA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type BA = asset::Ether;
                    #[allow(dead_code)]
                    type AH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type BH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type AI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type BI = identity::Ethereum;
                    #[allow(dead_code)]
                    type AT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type BT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRefund<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
                BitcoinLedgerKind::Regtest => {
                    #[allow(dead_code)]
                    type AL = bitcoin::Regtest;
                    #[allow(dead_code)]
                    type BL = Ethereum;
                    #[allow(dead_code)]
                    type AA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type BA = asset::Ether;
                    #[allow(dead_code)]
                    type AH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type BH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type AI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type BI = identity::Ethereum;
                    #[allow(dead_code)]
                    type AT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type BT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRefund<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
            },
            SwapTypes {
                alpha_ledger: LedgerKind::Bitcoin(bitcoin_ledger_kind),
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Erc20,
                ..
            } => match bitcoin_ledger_kind {
                BitcoinLedgerKind::Testnet => {
                    #[allow(dead_code)]
                    type AL = bitcoin::Testnet;
                    #[allow(dead_code)]
                    type BL = Ethereum;
                    #[allow(dead_code)]
                    type AA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type BA = asset::Erc20;
                    #[allow(dead_code)]
                    type AH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type BH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type AI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type BI = identity::Ethereum;
                    #[allow(dead_code)]
                    type AT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type BT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRefund<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
                BitcoinLedgerKind::Regtest => {
                    #[allow(dead_code)]
                    type AL = bitcoin::Regtest;
                    #[allow(dead_code)]
                    type BL = Ethereum;
                    #[allow(dead_code)]
                    type AA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type BA = asset::Erc20;
                    #[allow(dead_code)]
                    type AH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type BH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type AI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type BI = identity::Ethereum;
                    #[allow(dead_code)]
                    type AT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type BT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRefund<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
                BitcoinLedgerKind::Mainnet => {
                    #[allow(dead_code)]
                    type AL = bitcoin::Mainnet;
                    #[allow(dead_code)]
                    type BL = Ethereum;
                    #[allow(dead_code)]
                    type AA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type BA = asset::Erc20;
                    #[allow(dead_code)]
                    type AH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type BH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type AI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type BI = identity::Ethereum;
                    #[allow(dead_code)]
                    type AT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type BT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRefund<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
            },

            SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::Bitcoin(bitcoin_ledger_kind),
                alpha_asset: AssetKind::Ether,
                beta_asset: AssetKind::Bitcoin,
                ..
            } => match bitcoin_ledger_kind {
                BitcoinLedgerKind::Mainnet => {
                    #[allow(dead_code)]
                    type AL = Ethereum;
                    #[allow(dead_code)]
                    type BL = bitcoin::Mainnet;
                    #[allow(dead_code)]
                    type AA = asset::Ether;
                    #[allow(dead_code)]
                    type BA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type AH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type BH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type AI = identity::Ethereum;
                    #[allow(dead_code)]
                    type BI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type AT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type BT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRedeem<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
                BitcoinLedgerKind::Testnet => {
                    #[allow(dead_code)]
                    type AL = Ethereum;
                    #[allow(dead_code)]
                    type BL = bitcoin::Testnet;
                    #[allow(dead_code)]
                    type AA = asset::Ether;
                    #[allow(dead_code)]
                    type BA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type AH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type BH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type AI = identity::Ethereum;
                    #[allow(dead_code)]
                    type BI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type AT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type BT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRedeem<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
                BitcoinLedgerKind::Regtest => {
                    #[allow(dead_code)]
                    type AL = Ethereum;
                    #[allow(dead_code)]
                    type BL = bitcoin::Regtest;
                    #[allow(dead_code)]
                    type AA = asset::Ether;
                    #[allow(dead_code)]
                    type BA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type AH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type BH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type AI = identity::Ethereum;
                    #[allow(dead_code)]
                    type BI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type AT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type BT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRedeem<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
            },
            SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::Bitcoin(bitcoin_ledger_kind),
                alpha_asset: AssetKind::Erc20,
                beta_asset: AssetKind::Bitcoin,
                ..
            } => match bitcoin_ledger_kind {
                BitcoinLedgerKind::Mainnet => {
                    #[allow(dead_code)]
                    type AL = Ethereum;
                    #[allow(dead_code)]
                    type BL = bitcoin::Mainnet;
                    #[allow(dead_code)]
                    type AA = asset::Erc20;
                    #[allow(dead_code)]
                    type BA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type AI = identity::Ethereum;
                    #[allow(dead_code)]
                    type BI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type AH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type BH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type AT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type BT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRedeem<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
                BitcoinLedgerKind::Testnet => {
                    #[allow(dead_code)]
                    type AL = Ethereum;
                    #[allow(dead_code)]
                    type BL = bitcoin::Testnet;
                    #[allow(dead_code)]
                    type AA = asset::Erc20;
                    #[allow(dead_code)]
                    type BA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type AH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type BH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type AI = identity::Ethereum;
                    #[allow(dead_code)]
                    type BI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type AT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type BT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRedeem<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
                BitcoinLedgerKind::Regtest => {
                    #[allow(dead_code)]
                    type AL = Ethereum;
                    #[allow(dead_code)]
                    type BL = bitcoin::Regtest;
                    #[allow(dead_code)]
                    type AA = asset::Erc20;
                    #[allow(dead_code)]
                    type BA = asset::Bitcoin;
                    #[allow(dead_code)]
                    type AH = htlc_location::Ethereum;
                    #[allow(dead_code)]
                    type BH = htlc_location::Bitcoin;
                    #[allow(dead_code)]
                    type AI = identity::Ethereum;
                    #[allow(dead_code)]
                    type BI = identity::Bitcoin;
                    #[allow(dead_code)]
                    type AT = transaction::Ethereum;
                    #[allow(dead_code)]
                    type BT = transaction::Bitcoin;
                    #[allow(dead_code)]
                    type AcceptBody =
                        crate::http_api::routes::rfc003::accept::OnlyRedeem<identity::Ethereum>;

                    _match_role!(role, $fn)
                }
            },
            _ => unimplemented!(),
        }
    }};
}
