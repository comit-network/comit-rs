#[macro_export]
macro_rules! get_swap {
    ($metadata_store:expr, $state_store:expr, $id:expr, $result:ident, success: $fn:tt, failure: $failure:expr) => {{
        let metadata_store = $metadata_store;
        let state_store = $state_store;
        let id = $id;

        match metadata_store.get(&id) {
            Err(e) => $failure(e),
            Ok(
                metadata @ Metadata {
                    alpha_ledger: LedgerKind::Bitcoin,
                    beta_ledger: LedgerKind::Ethereum,
                    alpha_asset: AssetKind::Bitcoin,
                    beta_asset: AssetKind::Ether,
                    ..
                },
            ) => {
                info!("Fetched metadata of swap with id {}: {:?}", id, metadata);
                match metadata.role {
                    RoleKind::Alice => {
                        let $result = (
                            state_store
                                .get::<Alice<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>>(id)
                                .unwrap(),
                            metadata,
                        );
                        $fn()
                    }
                    RoleKind::Bob => {
                        let $result = (
                            state_store
                                .get::<Bob<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>>(id)
                                .unwrap(),
                            metadata,
                        );
                        $fn()
                    }
                }
            }
            _ => unreachable!("No other type is expected to be found in the store"),
        }
    }};
}
