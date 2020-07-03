//! Execute a swap.

mod alice;
mod bitcoin;
mod bob;
mod db;
mod ethereum;
mod hbit;
mod herc20;

use futures::future;

pub use alice::WatchOnlyAlice;
pub use bob::WalletBob;
use comit::Timestamp;

/// Execute a Hbit<->Herc20 swap.
pub async fn hbit_herc20<A, B>(
    alice: A,
    bob: B,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
) -> anyhow::Result<()>
where
    A: Do<hbit::CorrectlyFunded>
        + Execute<hbit::CorrectlyFunded, Args = hbit::Params>
        + Do<herc20::Redeemed>
        + Execute<herc20::Redeemed, Args = (herc20::Params, herc20::Deployed)>
        + hbit::Refund
        + Sync,
    B: Do<herc20::Deployed>
        + Execute<herc20::Deployed, Args = herc20::Params>
        + Do<herc20::CorrectlyFunded>
        + Execute<herc20::CorrectlyFunded, Args = (herc20::Params, herc20::Deployed)>
        + hbit::RedeemAsBob
        + herc20::Refund
        + Sync,
{
    let beta_expiry = herc20_params.expiry;
    let hbit_funded =
        match Do::<hbit::CorrectlyFunded>::r#do(&alice, beta_expiry, hbit_params).await? {
            Next::Continue(hbit_funded) => hbit_funded,
            Next::Abort => return Ok(()),
        };

    let herc20_deployed =
        match Do::<herc20::Deployed>::r#do(&bob, beta_expiry, herc20_params.clone()).await? {
            Next::Continue(herc20_deployed) => herc20_deployed,
            Next::Abort => {
                alice.refund(&hbit_params, hbit_funded).await?;

                return Ok(());
            }
        };

    let _herc20_funded = match Do::<herc20::CorrectlyFunded>::r#do(
        &bob,
        beta_expiry,
        (herc20_params.clone(), herc20_deployed.clone()),
    )
    .await?
    {
        Next::Continue(herc20_funded) => herc20_funded,
        Next::Abort => {
            alice.refund(&hbit_params, hbit_funded).await?;

            return Ok(());
        }
    };

    let herc20_redeemed = match Do::<herc20::Redeemed>::r#do(
        &alice,
        beta_expiry,
        (herc20_params.clone(), herc20_deployed.clone()),
    )
    .await?
    {
        Next::Continue(herc20_redeemed) => herc20_redeemed,
        Next::Abort => {
            alice.refund(&hbit_params, hbit_funded).await?;
            bob.refund(herc20_params, herc20_deployed.clone()).await?;

            return Ok(());
        }
    };

    let hbit_redeem = bob.redeem(&hbit_params, hbit_funded, herc20_redeemed.secret);
    let hbit_refund = alice.refund(&hbit_params, hbit_funded);

    // It's always safe for Bob to redeem, he just has to do it before
    // Alice refunds
    match future::try_select(hbit_redeem, hbit_refund).await {
        Ok(future::Either::Left((_hbit_redeemed, _))) => Ok(()),
        Ok(future::Either::Right((_hbit_refunded, _))) => Ok(()),
        Err(either) => {
            let (error, _other_future) = either.factor_first();
            Err(error)
        }
    }
}

#[async_trait::async_trait]
pub trait Do<E>
where
    Self: CheckMemory<E> + ShouldAbort + Execute<E> + Remember<E>,
    E: Clone + Send + Sync + 'static,
    <Self as Execute<E>>::Args: Send + Sync,
{
    async fn r#do(
        &self,
        beta_expiry: Timestamp,
        execution_args: <Self as Execute<E>>::Args,
    ) -> anyhow::Result<Next<E>> {
        if let Some(event) = self.check_memory().await? {
            return Ok(Next::Continue(event));
        }

        if self.should_abort(beta_expiry).await? {
            return Ok(Next::Abort);
        }

        let event = Execute::<E>::execute(self, execution_args).await?;
        self.remember(event.clone()).await?;

        Ok(Next::Continue(event))
    }
}

#[async_trait::async_trait]
impl<E, A> Do<E> for A
where
    A: CheckMemory<E> + ShouldAbort + Execute<E> + Remember<E>,
    E: Clone + Send + Sync + 'static,
    <Self as Execute<E>>::Args: Send + Sync,
{
}

#[async_trait::async_trait]
pub trait CheckMemory<E> {
    async fn check_memory(&self) -> anyhow::Result<Option<E>>;
}

// NOTE: Currently, we only implement this trait once per actor, and
// we reuse its logic for all implementations of the Do trait
#[async_trait::async_trait]
pub trait ShouldAbort {
    async fn should_abort(&self, beta_expiry: Timestamp) -> anyhow::Result<bool>;
}

#[async_trait::async_trait]
pub trait Execute<E> {
    type Args;
    async fn execute(&self, args: Self::Args) -> anyhow::Result<E>;
}

#[async_trait::async_trait]
pub trait Remember<E> {
    async fn remember(&self, event: E) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub enum Next<E> {
    Continue(E),
    Abort,
}

#[async_trait::async_trait]
pub trait BlockchainTime {
    async fn blockchain_time(&self) -> anyhow::Result<Timestamp>;
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::{
        bitcoin_wallet, ethereum_wallet,
        swap::{alice::wallet_actor::WalletAlice, bitcoin, bob::watch_only_actor::WatchOnlyBob},
        test_harness, Seed, SwapId,
    };
    use ::bitcoin::secp256k1;
    use chrono::Utc;
    use comit::{
        asset::{
            self,
            ethereum::{Erc20Quantity, FromWei},
        },
        btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
        identity, Secret, SecretHash, Timestamp,
    };
    use std::{str::FromStr, sync::Arc};
    use testcontainers::clients;

    fn hbit_params(
        secret_hash: SecretHash,
        network: ::bitcoin::Network,
        final_refund_identity: ::bitcoin::Address,
        final_redeem_identity: ::bitcoin::Address,
    ) -> (
        hbit::Params,
        hbit::PrivateDetailsFunder,
        hbit::PrivateDetailsRedeemer,
    ) {
        let asset = asset::Bitcoin::from_sat(100_000_000);
        let expiry = Timestamp::now().plus(60 * 60);

        let (private_details_funder, transient_refund_pk) = {
            let transient_refund_sk = secp256k1::SecretKey::from_str(
                "01010101010101010001020304050607ffff0000ffff00006363636363636363",
            )
            .unwrap();
            let private_details_funder = hbit::PrivateDetailsFunder {
                transient_refund_sk,
                final_refund_identity,
            };

            let transient_refund_pk =
                identity::Bitcoin::from_secret_key(&crate::SECP, &transient_refund_sk);

            (private_details_funder, transient_refund_pk)
        };

        let (private_details_redeemer, transient_redeem_pk) = {
            let transient_redeem_sk = secp256k1::SecretKey::from_str(
                "01010101010101010001020304050607ffff0000ffff00006363636363636363",
            )
            .unwrap();
            let private_details_redeemer = hbit::PrivateDetailsRedeemer {
                transient_redeem_sk,
                final_redeem_identity,
            };

            let transient_redeem_pk =
                identity::Bitcoin::from_secret_key(&crate::SECP, &transient_redeem_sk);

            (private_details_redeemer, transient_redeem_pk)
        };

        let params = hbit::Params {
            network,
            asset,
            redeem_identity: transient_redeem_pk,
            refund_identity: transient_refund_pk,
            expiry,
            secret_hash,
        };

        (params, private_details_funder, private_details_redeemer)
    }

    fn secret() -> Secret {
        let bytes = b"hello world, you are beautiful!!";
        Secret::from(*bytes)
    }

    // TODO: Implement these traits on a real database
    #[derive(Clone, Copy)]
    struct Database;

    #[async_trait::async_trait]
    impl db::Load<hbit::CorrectlyFunded> for Database {
        async fn load(&self, _swap_id: SwapId) -> anyhow::Result<Option<hbit::CorrectlyFunded>> {
            Ok(None)
        }
    }

    #[async_trait::async_trait]
    impl db::Load<herc20::Deployed> for Database {
        async fn load(&self, _swap_id: SwapId) -> anyhow::Result<Option<herc20::Deployed>> {
            Ok(None)
        }
    }

    #[async_trait::async_trait]
    impl db::Load<herc20::CorrectlyFunded> for Database {
        async fn load(&self, _swap_id: SwapId) -> anyhow::Result<Option<herc20::CorrectlyFunded>> {
            Ok(None)
        }
    }

    #[async_trait::async_trait]
    impl db::Load<herc20::Redeemed> for Database {
        async fn load(&self, _swap_id: SwapId) -> anyhow::Result<Option<herc20::Redeemed>> {
            Ok(None)
        }
    }

    #[async_trait::async_trait]
    impl db::Save<hbit::CorrectlyFunded> for Database {
        async fn save(
            &self,
            _event: hbit::CorrectlyFunded,
            _swap_id: SwapId,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl db::Save<herc20::Deployed> for Database {
        async fn save(&self, _event: herc20::Deployed, _swap_id: SwapId) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl db::Save<herc20::CorrectlyFunded> for Database {
        async fn save(
            &self,
            _event: herc20::CorrectlyFunded,
            _swap_id: SwapId,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl db::Save<herc20::Redeemed> for Database {
        async fn save(&self, _event: herc20::Redeemed, _swap_id: SwapId) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn execute_alice_hbit_herc20_swap() -> anyhow::Result<()> {
        let client = clients::Cli::default();

        let alice_db = Database;
        let bob_db = Database;

        let bitcoin_network = ::bitcoin::Network::Regtest;
        let (bitcoin_connector, bitcoind_url, bitcoin_blockchain) = {
            let blockchain = test_harness::bitcoin::Blockchain::new(&client)?;
            blockchain.init().await?;

            let node_url = blockchain.node_url.clone();

            (
                Arc::new(BitcoindConnector::new(
                    node_url.clone(),
                    ::bitcoin::Network::Regtest,
                )?),
                node_url,
                blockchain,
            )
        };
        let ethereum_chain_id = ethereum::ChainId::regtest();
        let (ethereum_connector, ethereum_node_url, ethereum_blockchain, token_contract) = {
            let mut blockchain = test_harness::ethereum::Blockchain::new(&client)?;
            blockchain.init().await?;

            let node_url = blockchain.node_url.clone();

            let token_contract = blockchain.token_contract()?;

            (
                Arc::new(Web3Connector::new(node_url.clone())),
                node_url,
                blockchain,
                token_contract,
            )
        };

        let (alice_bitcoin_wallet, alice_ethereum_wallet) = {
            let seed = Seed::default();
            let bitcoin_wallet = {
                let wallet =
                    bitcoin_wallet::Wallet::new(seed, bitcoind_url.clone(), bitcoin_network)?;
                wallet.init().await?;

                bitcoin_blockchain
                    .mint(
                        wallet.new_address().await?,
                        asset::Bitcoin::from_sat(1_000_000_000).into(),
                    )
                    .await?;

                wallet
            };
            let ethereum_wallet = ethereum_wallet::Wallet::new(seed, ethereum_node_url.clone())?;

            (
                bitcoin::Wallet {
                    inner: bitcoin_wallet,
                    connector: Arc::clone(&bitcoin_connector),
                },
                ethereum::Wallet {
                    inner: ethereum_wallet,
                    connector: Arc::clone(&ethereum_connector),
                },
            )
        };

        let (bob_bitcoin_wallet, bob_ethereum_wallet) = {
            let seed = Seed::default();
            let bitcoin_wallet = {
                let wallet =
                    bitcoin_wallet::Wallet::new(seed, bitcoind_url.clone(), bitcoin_network)?;
                wallet.init().await?;

                wallet
            };
            let ethereum_wallet = ethereum_wallet::Wallet::new(seed, ethereum_node_url)?;

            ethereum_blockchain
                .mint(
                    ethereum_wallet.account(),
                    asset::Erc20::new(token_contract, Erc20Quantity::from_wei(5_000_000_000u64)),
                    ethereum_chain_id,
                )
                .await?;

            (
                bitcoin::Wallet {
                    inner: bitcoin_wallet,
                    connector: Arc::clone(&bitcoin_connector),
                },
                ethereum::Wallet {
                    inner: ethereum_wallet,
                    connector: Arc::clone(&ethereum_connector),
                },
            )
        };

        let secret = secret();
        let secret_hash = SecretHash::new(secret);

        let start_of_swap = Utc::now().naive_local();
        let beta_expiry = Timestamp::now().plus(60 * 60);

        let (hbit_params, private_details_funder, private_details_redeemer) = {
            let redeem_address = bob_bitcoin_wallet.inner.new_address().await?;
            let refund_address = alice_bitcoin_wallet.inner.new_address().await?;

            hbit_params(secret_hash, bitcoin_network, refund_address, redeem_address)
        };

        let herc20_params = herc20::params(
            secret_hash,
            ethereum_chain_id,
            alice_ethereum_wallet.inner.account(),
            bob_ethereum_wallet.inner.account(),
            token_contract,
            beta_expiry,
        );

        let alice_swap = {
            let swap_id = SwapId::random();
            let alice = WalletAlice {
                alpha_wallet: alice_bitcoin_wallet.clone(),
                beta_wallet: alice_ethereum_wallet.clone(),
                db: alice_db,
                private_protocol_details: private_details_funder,
                secret,
                start_of_swap,
                swap_id,
            };
            let bob = WatchOnlyBob {
                alpha_connector: Arc::clone(&bitcoin_connector),
                beta_connector: Arc::clone(&ethereum_connector),
                db: alice_db,
                secret_hash,
                start_of_swap,
                swap_id,
            };

            hbit_herc20(alice, bob, hbit_params, herc20_params.clone())
        };

        let bob_swap = {
            let swap_id = SwapId::random();
            let alice = WatchOnlyAlice {
                alpha_connector: Arc::clone(&bitcoin_connector),
                beta_connector: Arc::clone(&ethereum_connector),
                db: bob_db,
                secret_hash,
                start_of_swap,
                swap_id,
            };
            let bob = WalletBob {
                alpha_wallet: bob_bitcoin_wallet.clone(),
                beta_wallet: bob_ethereum_wallet.clone(),
                db: bob_db,
                secret_hash,
                private_protocol_details: private_details_redeemer,
                start_of_swap,
                swap_id,
            };

            hbit_herc20(alice, bob, hbit_params, herc20_params.clone())
        };

        let alice_bitcoin_starting_balance = alice_bitcoin_wallet.inner.balance().await?;
        let bob_bitcoin_starting_balance = bob_bitcoin_wallet.inner.balance().await?;

        let alice_erc20_starting_balance = alice_ethereum_wallet
            .inner
            .erc20_balance(token_contract)
            .await?;
        let bob_erc20_starting_balance = bob_ethereum_wallet
            .inner
            .erc20_balance(token_contract)
            .await?;

        futures::future::try_join(alice_swap, bob_swap).await?;

        let alice_bitcoin_final_balance = alice_bitcoin_wallet.inner.balance().await?;
        let bob_bitcoin_final_balance = bob_bitcoin_wallet.inner.balance().await?;
        let bitcoin_max_fee = bitcoin::Amount::from_sat(100000);

        let alice_erc20_final_balance = alice_ethereum_wallet
            .inner
            .erc20_balance(token_contract)
            .await?;
        let bob_erc20_final_balance = bob_ethereum_wallet
            .inner
            .erc20_balance(token_contract)
            .await?;

        assert!(
            alice_bitcoin_final_balance
                >= alice_bitcoin_starting_balance - hbit_params.asset.into() - bitcoin_max_fee
        );
        assert!(
            bob_bitcoin_final_balance
                >= bob_bitcoin_starting_balance + hbit_params.asset.into() - bitcoin_max_fee
        );

        assert_eq!(
            alice_erc20_final_balance.quantity.to_u256(),
            alice_erc20_starting_balance.quantity.to_u256()
                + herc20_params.asset.quantity.to_u256()
        );
        assert_eq!(
            bob_erc20_final_balance.quantity.to_u256(),
            bob_erc20_starting_balance.quantity.to_u256() - herc20_params.asset.quantity.to_u256()
        );

        Ok(())
    }
}
