macro_rules! _match_role_bob {
    ($role:ident, $fn:tt) => {{
        use crate::swap_protocols::rfc003::bob;
        #[allow(clippy::redundant_closure_call)]
        match $role {
            RoleKind::Bob => {
                #[allow(dead_code)]
                type ROLE = bob::State<AL, BL, AA, BA>;
                $fn()
            }
            _ => Err(
                HttpApiProblem::with_title_and_type_from_status(HttpStatusCode::BAD_REQUEST)
                    .set_detail("Requested action is not supported for this role."),
            ),
        }
    }};
}

#[macro_export]
macro_rules! with_swap_types_bob {
    ($metadata:expr, $fn:tt) => {{
        use crate::swap_protocols::{asset::AssetKind, LedgerKind};
        use bitcoin_support::BitcoinQuantity;
        use ethereum_support::EtherQuantity;
        let metadata = $metadata;

        match metadata {
            Metadata {
                alpha_ledger: LedgerKind::Bitcoin(_),
                beta_ledger: LedgerKind::Ethereum(_),
                alpha_asset: AssetKind::Bitcoin(_),
                beta_asset: AssetKind::Ether(_),
                role,
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
                type BobAcceptBody = OnlyRefund<BL>;

                _match_role_bob!(role, $fn)
            }
            Metadata {
                alpha_ledger: LedgerKind::Bitcoin(_),
                beta_ledger: LedgerKind::Ethereum(_),
                alpha_asset: AssetKind::Bitcoin(_),
                beta_asset: AssetKind::Erc20(_),
                role,
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
                type BobAcceptBody = OnlyRefund<BL>;

                _match_role_bob!(role, $fn)
            }
            Metadata {
                alpha_ledger: LedgerKind::Ethereum(_),
                beta_ledger: LedgerKind::Bitcoin(_),
                alpha_asset: AssetKind::Ether(_),
                beta_asset: AssetKind::Bitcoin(_),
                role,
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
                type BobAcceptBody = OnlyRedeem<AL>;

                _match_role_bob!(role, $fn)
            }
            Metadata {
                alpha_ledger: LedgerKind::Ethereum(_),
                beta_ledger: LedgerKind::Bitcoin(_),
                alpha_asset: AssetKind::Erc20(_),
                beta_asset: AssetKind::Bitcoin(_),
                role,
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
                type BobAcceptBody = OnlyRedeem<AL>;

                _match_role_bob!(role, $fn)
            }
            _ => unimplemented!(),
        }
    }};
}
