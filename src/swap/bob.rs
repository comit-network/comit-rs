//! Bob's perspective of the swap.
//!
//! In Nectar we always take the role of Bob in the swap, so this
//! component has to be prepared to execute actions using wallets.

use crate::{
    swap::{
        bitcoin, db, ethereum, BlockchainTime, CheckMemory, Execute, Next, Remember, ShouldAbort,
        {hbit, herc20},
    },
    SwapId,
};
use chrono::NaiveDateTime;
use comit::{Secret, SecretHash, Timestamp};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct WalletBob<AW, BW, DB, E> {
    pub alpha_wallet: AW,
    pub beta_wallet: BW,
    pub db: DB,
    pub private_protocol_details: E,
    pub secret_hash: SecretHash,
    pub start_of_swap: NaiveDateTime,
    pub swap_id: SwapId,
}

#[async_trait::async_trait]
impl<AW, BW, DB, E> CheckMemory<herc20::Deployed> for WalletBob<AW, BW, DB, E>
where
    AW: Send + Sync,
    BW: Send + Sync,
    DB: db::Load<herc20::Deployed>,
    E: Send + Sync,
{
    async fn check_memory(&self) -> anyhow::Result<Option<herc20::Deployed>> {
        self.db.load(self.swap_id).await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, E> ShouldAbort for WalletBob<AW, BW, DB, E>
where
    AW: Send + Sync,
    BW: BlockchainTime + Send + Sync,
    DB: Send + Sync,
    E: Send + Sync,
{
    async fn should_abort(&self, beta_expiry: Timestamp) -> anyhow::Result<bool> {
        let beta_blockchain_time = self.beta_wallet.blockchain_time().await?;

        Ok(beta_expiry <= beta_blockchain_time)
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, E> Execute<herc20::Deployed> for WalletBob<AW, BW, DB, E>
where
    AW: Send + Sync,
    BW: herc20::ExecuteDeploy + Send + Sync,
    DB: Send + Sync,
    E: Send + Sync,
{
    type Args = herc20::Params;

    async fn execute(&self, params: herc20::Params) -> anyhow::Result<herc20::Deployed> {
        self.beta_wallet.execute_deploy(params).await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, E> Remember<herc20::Deployed> for WalletBob<AW, BW, DB, E>
where
    AW: Send + Sync,
    BW: Send + Sync,
    DB: db::Save<herc20::Deployed>,
    E: Send + Sync,
{
    async fn remember(&self, event: herc20::Deployed) -> anyhow::Result<()> {
        self.db.save(event, self.swap_id).await
    }
}

#[async_trait::async_trait]
impl<AW, DB, E> herc20::Fund for WalletBob<AW, ethereum::Wallet, DB, E>
where
    AW: Send + Sync,
    DB: db::Load<herc20::CorrectlyFunded> + db::Save<herc20::CorrectlyFunded>,
    E: Send + Sync,
{
    async fn fund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        beta_expiry: Timestamp,
    ) -> anyhow::Result<Next<herc20::CorrectlyFunded>> {
        if let Some(fund_event) = self.db.load(self.swap_id).await? {
            return Ok(Next::Continue(fund_event));
        }

        if beta_expiry <= self.beta_wallet.blockchain_time().await? {
            return Ok(Next::Abort);
        }

        let fund_event = self.fund(params, deploy_event).await?;
        self.db.save(fund_event.clone(), self.swap_id).await?;

        Ok(Next::Continue(fund_event))
    }
}

#[async_trait::async_trait]
impl<DB> hbit::RedeemAsBob
    for WalletBob<bitcoin::Wallet, ethereum::Wallet, DB, hbit::PrivateDetailsRedeemer>
where
    DB: Send + Sync,
{
    async fn redeem(
        &self,
        params: &hbit::Params,
        fund_event: hbit::CorrectlyFunded,
        secret: Secret,
    ) -> anyhow::Result<hbit::Redeemed> {
        self.redeem(*params, fund_event, secret).await
    }
}

#[async_trait::async_trait]
impl<DB> herc20::Refund
    for WalletBob<bitcoin::Wallet, ethereum::Wallet, DB, hbit::PrivateDetailsRedeemer>
where
    DB: Send + Sync,
{
    async fn refund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::Refunded> {
        loop {
            if self.beta_wallet.blockchain_time().await? >= params.expiry {
                break;
            }

            tokio::time::delay_for(Duration::from_secs(1)).await;
        }

        let refund_event = self.refund(params, deploy_event).await?;

        Ok(refund_event)
    }
}

impl<AW, DB, E> WalletBob<AW, ethereum::Wallet, DB, E> {
    async fn fund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::CorrectlyFunded> {
        let fund_action = params.build_fund_action(deploy_event.location);
        self.beta_wallet.fund(fund_action).await?;

        let event = herc20::watch_for_funded(
            self.beta_wallet.connector.as_ref(),
            params,
            self.start_of_swap,
            deploy_event,
        )
        .await?;

        Ok(event)
    }

    async fn refund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::Refunded> {
        let refund_action = params.build_refund_action(deploy_event.location);
        self.beta_wallet.refund(refund_action).await?;

        let refund_event = herc20::watch_for_refunded(
            self.beta_wallet.connector.as_ref(),
            self.start_of_swap,
            deploy_event,
        )
        .await?;

        Ok(refund_event)
    }
}

impl<BW, DB> WalletBob<bitcoin::Wallet, BW, DB, hbit::PrivateDetailsRedeemer> {
    async fn redeem(
        &self,
        params: hbit::Params,
        fund_event: hbit::CorrectlyFunded,
        secret: Secret,
    ) -> anyhow::Result<hbit::Redeemed> {
        let redeem_action = params.build_redeem_action(
            &crate::SECP,
            fund_event.asset,
            fund_event.location,
            self.private_protocol_details.clone().transient_redeem_sk,
            self.private_protocol_details.clone().final_redeem_identity,
            secret,
        )?;
        let transaction = self.alpha_wallet.redeem(redeem_action).await?;
        let redeem_event = hbit::Redeemed {
            transaction,
            secret,
        };

        Ok(redeem_event)
    }
}

#[cfg(test)]
pub mod watch_only_actor {
    //! This module is only useful for integration tests, given that
    //! Nectar always executes a swap as Bob.

    use super::*;
    use comit::btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock};
    use std::sync::Arc;

    #[derive(Clone, Debug)]
    pub struct WatchOnlyBob<AC, BC, DB> {
        pub alpha_connector: Arc<AC>,
        pub beta_connector: Arc<BC>,
        pub db: DB,
        pub secret_hash: SecretHash,
        pub start_of_swap: NaiveDateTime,
        pub swap_id: SwapId,
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB> CheckMemory<herc20::Deployed> for WatchOnlyBob<AC, BC, DB>
    where
        AC: Send + Sync,
        BC: Send + Sync,
        DB: db::Load<herc20::Deployed>,
    {
        async fn check_memory(&self) -> anyhow::Result<Option<herc20::Deployed>> {
            self.db.load(self.swap_id).await
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB> ShouldAbort for WatchOnlyBob<AC, BC, DB>
    where
        AC: Send + Sync,
        BC: BlockchainTime + Send + Sync,
        DB: Send + Sync,
    {
        async fn should_abort(&self, beta_expiry: Timestamp) -> anyhow::Result<bool> {
            let beta_blockchain_time = self.beta_connector.as_ref().blockchain_time().await?;

            Ok(beta_expiry <= beta_blockchain_time)
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB> Execute<herc20::Deployed> for WatchOnlyBob<AC, BC, DB>
    where
        AC: Send + Sync,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: Send + Sync,
    {
        type Args = herc20::Params;

        async fn execute(&self, params: herc20::Params) -> anyhow::Result<herc20::Deployed> {
            herc20::watch_for_deployed(self.beta_connector.as_ref(), params, self.start_of_swap)
                .await
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB> Remember<herc20::Deployed> for WatchOnlyBob<AC, BC, DB>
    where
        AC: Send + Sync,
        BC: Send + Sync,
        DB: db::Save<herc20::Deployed>,
    {
        async fn remember(&self, event: herc20::Deployed) -> anyhow::Result<()> {
            self.db.save(event, self.swap_id).await
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB> herc20::Fund for WatchOnlyBob<AC, BC, DB>
    where
        AC: Send + Sync,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: db::Load<herc20::CorrectlyFunded> + db::Save<herc20::CorrectlyFunded>,
    {
        async fn fund(
            &self,
            params: herc20::Params,
            deploy_event: herc20::Deployed,
            beta_expiry: Timestamp,
        ) -> anyhow::Result<Next<herc20::CorrectlyFunded>> {
            {
                if let Some(fund_event) = self.db.load(self.swap_id).await? {
                    return Ok(Next::Continue(fund_event));
                }

                if beta_expiry <= self.beta_connector.as_ref().blockchain_time().await? {
                    return Ok(Next::Abort);
                }

                let fund_event = herc20::watch_for_funded(
                    self.beta_connector.as_ref(),
                    params,
                    self.start_of_swap,
                    deploy_event,
                )
                .await?;
                self.db.save(fund_event.clone(), self.swap_id).await?;

                Ok(Next::Continue(fund_event))
            }
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB> hbit::RedeemAsBob for WatchOnlyBob<AC, BC, DB>
    where
        AC: LatestBlock<Block = bitcoin::Block>
            + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: Send + Sync,
    {
        async fn redeem(
            &self,
            params: &hbit::Params,
            fund_event: hbit::CorrectlyFunded,
            _secret: Secret,
        ) -> anyhow::Result<hbit::Redeemed> {
            let event = hbit::watch_for_redeemed(
                self.alpha_connector.as_ref(),
                &params,
                fund_event.location,
                self.start_of_swap,
            )
            .await?;

            Ok(event)
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB> herc20::Refund for WatchOnlyBob<AC, BC, DB>
    where
        AC: Send + Sync,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: Send + Sync,
    {
        async fn refund(
            &self,
            _params: herc20::Params,
            deploy_event: herc20::Deployed,
        ) -> anyhow::Result<herc20::Refunded> {
            let event = herc20::watch_for_refunded(
                self.beta_connector.as_ref(),
                self.start_of_swap,
                deploy_event,
            )
            .await?;

            Ok(event)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swap::Do;
    use comit::{htlc_location, identity, transaction};
    use primitive_types::U256;
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
    };

    #[derive(Default)]
    struct MockDatabase {
        deploy_events: Arc<RwLock<HashMap<SwapId, herc20::Deployed>>>,
    }

    struct MockBitcoinWallet;

    struct MockEthereumWallet {
        node: Arc<RwLock<EthereumBlockchain>>,
    }

    #[derive(Default)]
    struct EthereumBlockchain {
        deploy_events: Vec<herc20::Deployed>,
    }

    struct Lock {
        secret: Secret,
        secret_hash: SecretHash,
    }

    impl Lock {
        fn new() -> Self {
            let bytes = b"hello world, you are beautiful!!";
            let secret = Secret::from(*bytes);

            let secret_hash = SecretHash::new(secret);

            Self {
                secret,
                secret_hash,
            }
        }
    }

    struct SwapTimes {
        start_of_swap: NaiveDateTime,
        alpha_expiry: Timestamp,
        beta_expiry: Timestamp,
    }

    impl SwapTimes {
        fn live_swap() -> Self {
            let start_of_swap = Timestamp::now();
            let beta_expiry = start_of_swap.plus(60 * 60);
            let alpha_expiry = beta_expiry.plus(60 * 60);

            Self {
                start_of_swap: start_of_swap.into(),
                alpha_expiry,
                beta_expiry,
            }
        }
    }

    #[async_trait::async_trait]
    impl herc20::ExecuteDeploy for MockEthereumWallet {
        async fn execute_deploy(
            &self,
            _params: herc20::Params,
        ) -> anyhow::Result<herc20::Deployed> {
            let deploy_event = herc20::Deployed {
                transaction: transaction::Ethereum {
                    hash: ethereum::Hash::from([0u8; 32]),
                    to: None,
                    value: U256::from(0u64),
                    input: Vec::new(),
                },
                location: htlc_location::Ethereum::random(),
            };

            let mut blockchain = self.node.write().unwrap();
            blockchain.deploy_events.push(deploy_event.clone());

            Ok(deploy_event)
        }
    }

    #[async_trait::async_trait]
    impl BlockchainTime for MockEthereumWallet {
        async fn blockchain_time(&self) -> anyhow::Result<Timestamp> {
            Ok(Timestamp::now())
        }
    }

    #[async_trait::async_trait]
    impl db::Load<herc20::Deployed> for MockDatabase {
        async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<herc20::Deployed>> {
            let deploy_events = self.deploy_events.read().unwrap();

            Ok(deploy_events.get(&swap_id).cloned())
        }
    }

    #[async_trait::async_trait]
    impl db::Save<herc20::Deployed> for MockDatabase {
        async fn save(
            &self,
            deploy_event: herc20::Deployed,
            swap_id: SwapId,
        ) -> anyhow::Result<()> {
            let mut deploy_events = self.deploy_events.write().unwrap();
            deploy_events.insert(swap_id, deploy_event);

            Ok(())
        }
    }

    #[tokio::test]
    async fn herc20_deploy_is_idempotent() {
        let ethereum_blockchain = Arc::new(RwLock::new(EthereumBlockchain::default()));
        let ethereum_wallet = MockEthereumWallet {
            node: Arc::clone(&ethereum_blockchain),
        };

        let db = MockDatabase::default();

        let swap_id = SwapId::random();

        let Lock { secret_hash, .. } = Lock::new();

        let SwapTimes {
            start_of_swap,
            beta_expiry,
            ..
        } = SwapTimes::live_swap();

        let herc20_params = herc20::params(
            secret_hash,
            ethereum::ChainId::regtest(),
            identity::Ethereum::random(),
            identity::Ethereum::random(),
            ethereum::Address::random(),
            beta_expiry,
        );

        let bob = WalletBob {
            alpha_wallet: MockBitcoinWallet,
            beta_wallet: ethereum_wallet,
            db,
            private_protocol_details: (),
            secret_hash,
            start_of_swap,
            swap_id,
        };

        assert!(ethereum_blockchain.read().unwrap().deploy_events.is_empty());
        let res = bob.r#do(beta_expiry, herc20_params.clone()).await;

        assert!(matches!(res, Ok(Next::Continue(herc20::Deployed { .. }))));
        assert_eq!(ethereum_blockchain.read().unwrap().deploy_events.len(), 1);

        let res = bob.r#do(beta_expiry, herc20_params).await;
        assert!(matches!(res, Ok(Next::Continue(herc20::Deployed { .. }))));
        assert_eq!(ethereum_blockchain.read().unwrap().deploy_events.len(), 1);
    }
}
