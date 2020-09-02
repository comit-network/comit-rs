//! Implement traits to Load/Save types defined in the http_api module.
use crate::{
    asset,
    http_api::{halbit, hbit, herc20, AliceSwap, BobSwap},
    state::Get,
    storage::{
        Halbit, Hbit, Herc20, Load, LoadTables, NoRedeemIdentity, NoRefundIdentity, NoSecretHash,
        RootSeed, Tables,
    },
    LocalSwapId, Storage,
};
use async_trait::async_trait;

/// Convert data from a protocol table, along with its associated state, into a
/// Finalized.
trait IntoFinalized {
    type Finalized;
    type State;

    fn into_finalized(self, state: Self::State) -> anyhow::Result<Self::Finalized>;
}

/// Convert data from the hbit protocol table, along with its associated state,
/// into a FinalizedAsRedeemer object.
trait IntoFinalizedAsRedeemer {
    fn into_finalized_as_redeemer(
        self,
        swap_id: LocalSwapId,
        seed: RootSeed,
        state: hbit::State,
    ) -> anyhow::Result<hbit::FinalizedAsRedeemer>;
}

/// Convert data from the hbit protocol table, along with its associated state,
/// into a FinalizedAsFunder object.
trait IntoFinalizedAsFunder {
    fn into_finalized_as_funder(
        self,
        swap_id: LocalSwapId,
        seed: RootSeed,
        state: hbit::State,
    ) -> anyhow::Result<hbit::FinalizedAsFunder>;
}

#[async_trait]
impl Load<AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>>
    for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>>
    {
        let alpha_state = self.herc20_states.get(&swap_id).await?;
        let beta_state = self.halbit_states.get(&swap_id).await?;

        let tab: Tables<Herc20, Halbit> = self.load_tables(swap_id).await?;

        let swap = match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => {
                let alpha_finalized = tab.alpha.into_finalized(alpha_state)?;
                let beta_finalized = tab.beta.into_finalized(beta_state)?;
                let secret = self.seed.derive_swap_seed(swap_id).derive_secret();

                AliceSwap::Finalized {
                    alpha_finalized,
                    beta_finalized,
                    secret,
                }
            }
            _ => AliceSwap::Created {
                alpha_created: tab.alpha.into(),
                beta_created: tab.beta.into(),
            },
        };

        Ok(swap)
    }
}

#[async_trait]
impl Load<AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>>
    for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>>
    {
        let alpha_state = self.halbit_states.get(&swap_id).await?;
        let beta_state = self.herc20_states.get(&swap_id).await?;

        let tab: Tables<Halbit, Herc20> = self.load_tables(swap_id).await?;

        let swap = match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => {
                let alpha_finalized = tab.alpha.into_finalized(alpha_state)?;
                let beta_finalized = tab.beta.into_finalized(beta_state)?;
                let secret = self.seed.derive_swap_seed(swap_id).derive_secret();

                AliceSwap::Finalized {
                    alpha_finalized,
                    beta_finalized,
                    secret,
                }
            }
            _ => AliceSwap::Created {
                alpha_created: tab.alpha.into(),
                beta_created: tab.beta.into(),
            },
        };

        Ok(swap)
    }
}

#[async_trait]
impl Load<AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>>
    for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>,
    > {
        let alpha_state = self.herc20_states.get(&swap_id).await?;
        let beta_state = self.hbit_states.get(&swap_id).await?;

        let tab: Tables<Herc20, Hbit> = self.load_tables(swap_id).await?;

        let swap = match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => {
                let alpha_finalized = tab.alpha.into_finalized(alpha_state)?;
                let beta_finalized = tab
                    .beta
                    .into_finalized_as_redeemer(swap_id, self.seed, beta_state)?;

                let secret = self.seed.derive_swap_seed(swap_id).derive_secret();

                AliceSwap::Finalized {
                    alpha_finalized,
                    beta_finalized,
                    secret,
                }
            }
            _ => AliceSwap::Created {
                alpha_created: tab.alpha.into(),
                beta_created: tab.beta.into(),
            },
        };

        Ok(swap)
    }
}

#[async_trait]
impl Load<AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>>
    for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>,
    > {
        let alpha_state = self.hbit_states.get(&swap_id).await?;
        let beta_state = self.herc20_states.get(&swap_id).await?;

        let tab: Tables<Hbit, Herc20> = self.load_tables(swap_id).await?;

        let swap = match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => {
                let alpha_finalized =
                    tab.alpha
                        .into_finalized_as_funder(swap_id, self.seed, alpha_state)?;
                let beta_finalized = tab.beta.into_finalized(beta_state)?;

                let secret = self.seed.derive_swap_seed(swap_id).derive_secret();

                AliceSwap::Finalized {
                    alpha_finalized,
                    beta_finalized,
                    secret,
                }
            }
            _ => AliceSwap::Created {
                alpha_created: tab.alpha.into(),
                beta_created: tab.beta.into(),
            },
        };

        Ok(swap)
    }
}

#[async_trait]
impl Load<BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>> for Storage {
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>>
    {
        let alpha_state = self.herc20_states.get(&swap_id).await?;
        let beta_state = self.halbit_states.get(&swap_id).await?;

        let tab: Tables<Herc20, Halbit> = self.load_tables(swap_id).await?;

        let swap = match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => {
                let alpha_finalized = tab.alpha.into_finalized(alpha_state)?;
                let beta_finalized = tab.beta.into_finalized(beta_state)?;

                let secret_hash = tab.secret_hash.ok_or(NoSecretHash(swap_id))?.secret_hash.0;

                BobSwap::Finalized {
                    alpha_finalized,
                    beta_finalized,
                    secret_hash,
                }
            }
            _ => BobSwap::Created {
                alpha_created: tab.alpha.into(),
                beta_created: tab.beta.into(),
            },
        };

        Ok(swap)
    }
}

#[async_trait]
impl Load<BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>> for Storage {
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>>
    {
        let alpha_state = self.halbit_states.get(&swap_id).await?;
        let beta_state = self.herc20_states.get(&swap_id).await?;

        let tab: Tables<Halbit, Herc20> = self.load_tables(swap_id).await?;

        let swap = match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => {
                let alpha_finalized = tab.alpha.into_finalized(alpha_state)?;
                let beta_finalized = tab.beta.into_finalized(beta_state)?;

                let secret_hash = tab.secret_hash.ok_or(NoSecretHash(swap_id))?.secret_hash.0;

                BobSwap::Finalized {
                    alpha_finalized,
                    beta_finalized,
                    secret_hash,
                }
            }
            _ => BobSwap::Created {
                alpha_created: tab.alpha.into(),
                beta_created: tab.beta.into(),
            },
        };

        Ok(swap)
    }
}

#[async_trait]
impl Load<BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>>
    for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>,
    > {
        let alpha_state = self.herc20_states.get(&swap_id).await?;
        let beta_state = self.hbit_states.get(&swap_id).await?;

        let tab: Tables<Herc20, Hbit> = self.load_tables(swap_id).await?;

        let swap = match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => {
                let alpha_finalized = tab.alpha.into_finalized(alpha_state)?;
                let beta_finalized = tab
                    .beta
                    .into_finalized_as_funder(swap_id, self.seed, beta_state)?;

                let secret_hash = tab.secret_hash.ok_or(NoSecretHash(swap_id))?.secret_hash.0;

                BobSwap::Finalized {
                    alpha_finalized,
                    beta_finalized,
                    secret_hash,
                }
            }
            _ => BobSwap::Created {
                alpha_created: tab.alpha.into(),
                beta_created: tab.beta.into(),
            },
        };

        Ok(swap)
    }
}

#[async_trait]
impl Load<BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>>
    for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>,
    > {
        let alpha_state = self.hbit_states.get(&swap_id).await?;
        let beta_state = self.herc20_states.get(&swap_id).await?;

        let tab: Tables<Hbit, Herc20> = self.load_tables(swap_id).await?;

        let swap = match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => {
                let beta_finalized = tab.beta.into_finalized(beta_state)?;
                let alpha_finalized =
                    tab.alpha
                        .into_finalized_as_redeemer(swap_id, self.seed, alpha_state)?;

                let secret_hash = tab.secret_hash.ok_or(NoSecretHash(swap_id))?.secret_hash.0;

                BobSwap::Finalized {
                    alpha_finalized,
                    beta_finalized,
                    secret_hash,
                }
            }
            _ => BobSwap::Created {
                alpha_created: tab.alpha.into(),
                beta_created: tab.beta.into(),
            },
        };

        Ok(swap)
    }
}

impl IntoFinalized for Herc20 {
    type Finalized = herc20::Finalized;
    type State = herc20::State;

    fn into_finalized(self, state: Self::State) -> anyhow::Result<Self::Finalized> {
        let asset = asset::Erc20 {
            quantity: self.amount.0.into(),
            token_contract: self.token_contract.0,
        };

        Ok(herc20::Finalized {
            asset,
            chain_id: self.chain_id.0.into(),
            refund_identity: self.refund_identity.ok_or(NoRefundIdentity)?.0,
            redeem_identity: self.redeem_identity.ok_or(NoRedeemIdentity)?.0,
            expiry: self.expiry.0.into(),
            state,
        })
    }
}

impl IntoFinalized for Halbit {
    type Finalized = halbit::Finalized;
    type State = halbit::State;

    fn into_finalized(self, state: Self::State) -> anyhow::Result<Self::Finalized> {
        Ok(halbit::Finalized {
            asset: self.amount.0.into(),
            network: self.network.0,
            refund_identity: self.refund_identity.ok_or(NoRefundIdentity)?.0,
            redeem_identity: self.redeem_identity.ok_or(NoRedeemIdentity)?.0,
            cltv_expiry: self.cltv_expiry.0.into(),
            state,
        })
    }
}

impl IntoFinalizedAsFunder for Hbit {
    fn into_finalized_as_funder(
        self,
        swap_id: LocalSwapId,
        seed: RootSeed,
        state: hbit::State,
    ) -> anyhow::Result<hbit::FinalizedAsFunder> {
        let finalized = hbit::FinalizedAsFunder {
            asset: self.amount.0.into(),
            network: self.network.0,
            transient_redeem_identity: self.transient_identity.ok_or(NoRedeemIdentity)?.0,
            transient_refund_identity: seed
                .derive_swap_seed(swap_id)
                .derive_transient_refund_identity(),
            final_refund_identity: self.final_identity.0,
            expiry: self.expiry.0.into(),
            state,
        };

        Ok(finalized)
    }
}

impl IntoFinalizedAsRedeemer for Hbit {
    fn into_finalized_as_redeemer(
        self,
        swap_id: LocalSwapId,
        seed: RootSeed,
        state: hbit::State,
    ) -> anyhow::Result<hbit::FinalizedAsRedeemer> {
        let finalized = hbit::FinalizedAsRedeemer {
            asset: self.amount.0.into(),
            network: self.network.0,
            transient_redeem_identity: seed
                .derive_swap_seed(swap_id)
                .derive_transient_redeem_identity(),
            transient_refund_identity: self.transient_identity.ok_or(NoRefundIdentity)?.0,
            final_redeem_identity: self.final_identity.0,
            expiry: self.expiry.0.into(),
            state,
        };

        Ok(finalized)
    }
}
