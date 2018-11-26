#[macro_export]
macro_rules! get_swap {
    ($metadata:expr, $state_store:expr, $id:expr, $state_name:ident, $found_fn:tt) => {{
        let metadata = $metadata;
        let state_store = $state_store;
        let id = $id;

        match metadata {
            metadata @ Metadata {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
                ..
            } => {
                info!("Fetched metadata of swap with id {}: {:?}", id, metadata);
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
            metadata @ Metadata {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::Bitcoin,
                alpha_asset: AssetKind::Erc20,
                beta_asset: AssetKind::Bitcoin,
                ..
            } => {
                info!("Fetched metadata of swap with id {}: {:?}", id, metadata);
                match metadata.role {
                    RoleKind::Alice => {
                        let state =
                            state_store
                                .get::<Alice<Ethereum, Bitcoin, Erc20Quantity, BitcoinQuantity>>(
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
                                .get::<Bob<Ethereum, Bitcoin, Erc20Quantity, BitcoinQuantity>>(id);

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
            _ => {
                error!("Found an unknown pair in the metadata for {}", id);
                Err(HttpApiProblem::with_title_and_type_from_status(500))
            }
        }
    }};
}
