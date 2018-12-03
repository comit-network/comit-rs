#[macro_export]
macro_rules! get_swap {
    ($metadata:expr, $state_store:expr, $id:expr, $state_name:ident, $found_fn:tt) => {{
        let metadata = $metadata;
        let state_store = $state_store;
        let id = $id;

        #[allow(clippy::redundant_closure_call)]
        match metadata {
            metadata @ Metadata {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
                ..
            } => {
                trace!("Fetched metadata of swap with id {}: {:?}", id, metadata);
                match metadata.role {
                    RoleKind::Alice => {
                        let state =
                            state_store
                                .get::<Alice<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>>(
                                    id,
                                );

                        match state {
                            Ok(state) => {
                                let $state_name = state;
                                $found_fn()
                            }
                            Err(e) => Err(e.into()),
                        }
                    }
                    RoleKind::Bob => {
                        let state =
                            state_store
                                .get::<Bob<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>>(id);

                        match state {
                            Ok(state) => {
                                let $state_name = state;
                                $found_fn()
                            }
                            Err(e) => Err(e.into()),
                        }
                    }
                }
            }
            metadata @ Metadata {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Erc20,
                ..
            } => {
                info!("Fetched metadata of swap with id {}: {:?}", id, metadata);
                match metadata.role {
                    RoleKind::Alice => {
                        let state =
                            state_store
                                .get::<Alice<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>>(
                                    id,
                                );

                        match state {
                            Ok(state) => {
                                let $state_name = state;
                                $found_fn()
                            }
                            Err(e) => Err(e.into()),
                        }
                    }
                    RoleKind::Bob => {
                        let state =
                            state_store
                                .get::<Bob<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>>(id);

                        match state {
                            Ok(state) => {
                                let $state_name = state;
                                $found_fn()
                            }
                            Err(e) => Err(e.into()),
                        }
                    }
                }
            }
            _ => Err(HttpApiProblem::with_title_and_type_from_status(500)
                .set_detail("Unknown metadata for swap")),
        }
    }};
}

macro_rules! _match_role {
    ($role:ident, $fn:tt) => {
        #[allow(clippy::redundant_closure_call)]
        match $role {
            RoleKind::Alice => {
                #[allow(dead_code)]
                type Role = Alice<AL, BL, AA, BA>;
                $fn()
            }
            RoleKind::Bob => {
                #[allow(dead_code)]
                type Role = Bob<AL, BL, AA, BA>;
                $fn()
            }
        }
    };
}

#[macro_export]
macro_rules! with_swap_types {
    ($metadata:expr, $fn:tt) => {{
        use bitcoin_support::BitcoinQuantity;
        use ethereum_support::EtherQuantity;
        use swap_protocols::rfc003::roles::{Alice, Bob};
        let metadata = $metadata;

        match metadata {
            Metadata {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
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

                _match_role!(role, $fn)
            }
            Metadata {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Erc20,
                role,
            } => {
                #[allow(dead_code)]
                type AL = Bitcoin;
                #[allow(dead_code)]
                type BL = Ethereum;
                #[allow(dead_code)]
                type AA = BitcoinQuantity;
                #[allow(dead_code)]
                type BA = Erc20Quantity;

                _match_role!(role, $fn)
            }
            _ => unimplemented!(),
        }
    }};
}
