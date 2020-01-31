use crate::{
    comit_api,
    db::{
        schema,
        wrapper_types::{custom_sql_types::Text, BitcoinNetwork},
        Sqlite,
    },
    diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl},
};
use async_trait::async_trait;
use comit::{
    asset,
    swap_protocols::{Role, SwapId},
};

/// Determine swap types for swaps currently stored in the database.
///
/// SwapTypes exists solely so we can use the with_swap_types!() macro to get
/// compile time types instead of generic types.
#[async_trait]
#[ambassador::delegatable_trait]
pub trait DetermineTypes: Send + Sync + 'static {
    async fn determine_types(&self, key: &SwapId) -> anyhow::Result<SwapTypes>;
}

#[async_trait]
impl DetermineTypes for Sqlite {
    async fn determine_types(&self, key: &SwapId) -> anyhow::Result<SwapTypes> {
        let role = self.role(key).await?;

        if let Some(bitcoin_network) = self
            .rfc003_bitcoin_ethereum_bitcoin_ether_request_messages_has_swap(key)
            .await?
        {
            return Ok(SwapTypes {
                alpha_ledger: bitcoin_network.into(),
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
                role,
            });
        }

        if let Some(bitcoin_network) = self
            .rfc003_ethereum_bitcoin_ether_bitcoin_request_messages_has_swap(key)
            .await?
        {
            return Ok(SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: bitcoin_network.into(),
                alpha_asset: AssetKind::Ether,
                beta_asset: AssetKind::Bitcoin,
                role,
            });
        }

        if let Some(bitcoin_network) = self
            .rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages_has_swap(key)
            .await?
        {
            return Ok(SwapTypes {
                alpha_ledger: bitcoin_network.into(),
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Erc20,
                role,
            });
        }

        if let Some(bitcoin_network) = self
            .rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages_has_swap(key)
            .await?
        {
            return Ok(SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: bitcoin_network.into(),
                alpha_asset: AssetKind::Erc20,
                beta_asset: AssetKind::Bitcoin,
                role,
            });
        }

        unreachable!("we got role for swap so the swap_id must exist")
    }
}

macro_rules! impl_has_swap {
    ($table:ident) => {
        paste::item! {
            async fn [<$table _has_swap>](&self, key: &SwapId) -> anyhow::Result<Option<BitcoinNetwork>> {
                use schema::$table as swaps;

                let record: Option<QueryableSwap> = self.do_in_transaction(|connection| {
                    let key = Text(key);
                    swaps::table
                        .filter(swaps::swap_id.eq(key))
                        .select((swaps::swap_id,swaps::bitcoin_network))
                        .first(&*connection)
                        .optional()
                })
                .await?;

                match record {
                    None => Ok(None),
                    Some(swap) => {
                        let network: BitcoinNetwork = *swap.bitcoin_network;
                        Ok(Some(network))
                    }
                }
            }
        }
    };
}

impl Sqlite {
    impl_has_swap!(rfc003_bitcoin_ethereum_bitcoin_ether_request_messages);
    impl_has_swap!(rfc003_ethereum_bitcoin_ether_bitcoin_request_messages);
    impl_has_swap!(rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages);
    impl_has_swap!(rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages);
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct QueryableSwap {
    swap_id: Text<SwapId>,
    bitcoin_network: Text<BitcoinNetwork>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwapTypes {
    pub alpha_ledger: LedgerKind,
    pub beta_ledger: LedgerKind,
    pub alpha_asset: AssetKind,
    pub beta_asset: AssetKind,
    pub role: Role,
}

#[derive(Debug, Clone, Copy, strum_macros::Display, strum_macros::EnumString, PartialEq)]
pub enum LedgerKind {
    BitcoinMainnet,
    BitcoinTestnet,
    BitcoinRegtest,
    Ethereum,
}

impl From<comit_api::LedgerKind> for LedgerKind {
    fn from(ledger: comit_api::LedgerKind) -> LedgerKind {
        match ledger {
            comit_api::LedgerKind::BitcoinMainnet => LedgerKind::BitcoinMainnet,
            comit_api::LedgerKind::BitcoinTestnet => LedgerKind::BitcoinTestnet,
            comit_api::LedgerKind::BitcoinRegtest => LedgerKind::BitcoinRegtest,
            comit_api::LedgerKind::Ethereum(_) => LedgerKind::Ethereum,
        }
    }
}

#[derive(Clone, Copy, Debug, strum_macros::Display, strum_macros::EnumString, PartialEq)]
pub enum AssetKind {
    Bitcoin,
    Ether,
    Erc20,
}

impl From<asset::AssetKind> for AssetKind {
    fn from(asset: asset::AssetKind) -> AssetKind {
        match asset {
            asset::AssetKind::Bitcoin(_) => AssetKind::Bitcoin,
            asset::AssetKind::Ether(_) => AssetKind::Ether,
            asset::AssetKind::Erc20(_) => AssetKind::Erc20,
        }
    }
}
