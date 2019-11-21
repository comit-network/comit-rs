use crate::{
    db::{custom_sql_types::Text, schema, Error, Sqlite},
    diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl},
    swap_protocols::{asset, ledger, Role, SwapId},
};
use async_trait::async_trait;
use strum_macros::{Display, EnumString};

/// Determine swap types for swaps currently stored in the database.
///
/// SwapTypes exists solely so we can use the with_swap_types!() macro to get
/// compile time types instead of generic types.
#[async_trait]
pub trait DetermineTypes: Send + Sync + 'static {
    async fn determine_types(&self, key: &SwapId) -> anyhow::Result<SwapTypes>;
}

#[async_trait]
impl DetermineTypes for Sqlite {
    async fn determine_types(&self, key: &SwapId) -> anyhow::Result<SwapTypes> {
        let role = self.role(key).await?;

        if self
            .rfc003_bitcoin_ethereum_bitcoin_ether_request_messages_has_swap(key)
            .await?
        {
            return Ok(SwapTypes {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
                role,
            });
        }

        if self
            .rfc003_ethereum_bitcoin_ether_bitcoin_request_messages_has_swap(key)
            .await?
        {
            return Ok(SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::Bitcoin,
                alpha_asset: AssetKind::Ether,
                beta_asset: AssetKind::Bitcoin,
                role,
            });
        }

        if self
            .rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages_has_swap(key)
            .await?
        {
            return Ok(SwapTypes {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Erc20,
                role,
            });
        }

        if self
            .rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages_has_swap(key)
            .await?
        {
            return Ok(SwapTypes {
                alpha_ledger: LedgerKind::Ethereum,
                beta_ledger: LedgerKind::Bitcoin,
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
            async fn [<$table _has_swap>](&self, key: &SwapId) -> anyhow::Result<bool> {
                use schema::$table as swaps;

                let connection = self.connect().await;
                let key = Text(key);

                let record: Result<QueryableSwap, Error> = swaps::table
                    .filter(swaps::swap_id.eq(key))
                    .select((swaps::swap_id,)) // Select call needs argument to be a tuple.
                    .first(&*connection)
                    .optional()?
                    .ok_or(Error::SwapNotFound);

                Ok(record.is_ok())
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwapTypes {
    pub alpha_ledger: LedgerKind,
    pub beta_ledger: LedgerKind,
    pub alpha_asset: AssetKind,
    pub beta_asset: AssetKind,
    pub role: Role,
}

#[derive(Debug, Clone, Display, EnumString, PartialEq)]
pub enum LedgerKind {
    Bitcoin,
    Ethereum,
}

impl From<ledger::LedgerKind> for LedgerKind {
    fn from(ledger: ledger::LedgerKind) -> LedgerKind {
        match ledger {
            ledger::LedgerKind::Bitcoin(_) => LedgerKind::Bitcoin,
            ledger::LedgerKind::Ethereum(_) => LedgerKind::Ethereum,
            // In order to remove this ledger::LedgerKind::Unknown should be removed.
            // Doing so requires handling unknown ledger during deserialization.
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, Display, EnumString, PartialEq)]
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
            // In order to remove this ledger::AssetKind::Unknown should be removed.
            // Doing so requires handling unknown asset during deserialization.
            _ => unreachable!(),
        }
    }
}
