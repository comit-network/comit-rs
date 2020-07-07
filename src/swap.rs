//! Execute a swap.

mod alice;
mod bitcoin;
mod bob;
mod db;
mod do_action;
mod ethereum;
mod hbit;
mod herc20;

use comit::Secret;
use futures::future;

pub use alice::WatchOnlyAlice;
pub use bob::WalletBob;
pub use do_action::{AlphaLedgerTime, BetaExpiry, BetaLedgerTime, Do, Execute, Next};

/// Execute a Hbit<->Herc20 swap.
pub async fn hbit_herc20<A, B>(alice: A, bob: B) -> anyhow::Result<()>
where
    A: Do<hbit::Funded>
        + Execute<hbit::Funded, Args = ()>
        + Do<herc20::Redeemed>
        + Execute<herc20::Redeemed, Args = herc20::Deployed>
        + Execute<hbit::Refunded, Args = hbit::Funded>
        + Sync,
    B: Do<herc20::Deployed>
        + Execute<herc20::Deployed, Args = ()>
        + Do<herc20::Funded>
        + Execute<herc20::Funded, Args = herc20::Deployed>
        + Execute<hbit::Redeemed, Args = (hbit::Funded, Secret)>
        + Execute<herc20::Refunded, Args = herc20::Deployed>
        + Sync,
{
    let hbit_funded = match Do::<hbit::Funded>::r#do(&alice, ()).await? {
        Next::Continue(hbit_funded) => hbit_funded,
        Next::Abort => return Ok(()),
    };

    let herc20_deployed = match Do::<herc20::Deployed>::r#do(&bob, ()).await? {
        Next::Continue(herc20_deployed) => herc20_deployed,
        Next::Abort => {
            Execute::<hbit::Refunded>::execute(&alice, hbit_funded).await?;

            return Ok(());
        }
    };

    let _herc20_funded = match Do::<herc20::Funded>::r#do(&bob, herc20_deployed.clone()).await? {
        Next::Continue(herc20_funded) => herc20_funded,
        Next::Abort => {
            Execute::<hbit::Refunded>::execute(&alice, hbit_funded).await?;

            return Ok(());
        }
    };

    let herc20_redeemed =
        match Do::<herc20::Redeemed>::r#do(&alice, herc20_deployed.clone()).await? {
            Next::Continue(herc20_redeemed) => herc20_redeemed,
            Next::Abort => {
                Execute::<hbit::Refunded>::execute(&alice, hbit_funded).await?;
                Execute::<herc20::Refunded>::execute(&bob, herc20_deployed.clone()).await?;

                return Ok(());
            }
        };

    // TODO: Prevent Bob from trying to redeem again (applies to the
    // all the refunds too). Reusing the Do trait seems wrong since we
    // should never abort at this stage, which is why we used the
    // Execute trait directly. There is no risk in doing this action
    // more than once, but it's a bit wasteful. We should probably
    // introduce another trait which composes CheckMemory, Execute and
    // Remember to solve this problem (P.S. naming is hard)
    let hbit_redeem =
        Execute::<hbit::Redeemed>::execute(&bob, (hbit_funded, herc20_redeemed.secret));
    let hbit_refund = Execute::<hbit::Refunded>::execute(&alice, hbit_funded);

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

/// Execute a Hbit<->Herc20 swap.
pub async fn herc20_hbit<A, B>(alice: A, bob: B) -> anyhow::Result<()>
where
    A: Do<herc20::Deployed>
        + Execute<herc20::Deployed, Args = ()>
        + Do<herc20::Funded>
        + Execute<herc20::Funded, Args = herc20::Deployed>
        + Do<hbit::Redeemed>
        + Execute<hbit::Redeemed, Args = hbit::Funded>
        + Execute<herc20::Refunded, Args = herc20::Deployed>
        + Sync,
    B: Do<hbit::Funded>
        + Execute<hbit::Funded, Args = ()>
        + Do<herc20::Redeemed>
        + Execute<herc20::Redeemed, Args = (herc20::Deployed, Secret)>
        + Execute<hbit::Refunded, Args = hbit::Funded>
        + Sync,
{
    let herc20_deployed = match Do::<herc20::Deployed>::r#do(&alice, ()).await? {
        Next::Continue(herc20_deployed) => herc20_deployed,
        Next::Abort => {
            return Ok(());
        }
    };

    let _herc20_funded = match Do::<herc20::Funded>::r#do(&alice, herc20_deployed.clone()).await? {
        Next::Continue(herc20_funded) => herc20_funded,
        Next::Abort => {
            return Ok(());
        }
    };

    let hbit_funded = match Do::<hbit::Funded>::r#do(&bob, ()).await? {
        Next::Continue(hbit_funded) => hbit_funded,
        Next::Abort => {
            Execute::<herc20::Refunded>::execute(&alice, herc20_deployed.clone()).await?;

            return Ok(());
        }
    };

    let hbit_redeemed = match Do::<hbit::Redeemed>::r#do(&alice, hbit_funded).await? {
        Next::Continue(hbit_redeemed) => hbit_redeemed,
        Next::Abort => {
            Execute::<herc20::Refunded>::execute(&alice, herc20_deployed.clone()).await?;
            Execute::<hbit::Refunded>::execute(&bob, hbit_funded).await?;

            return Ok(());
        }
    };

    // TODO: Prevent Bob from trying to redeem again (applies to the
    // all the refunds too). Reusing the Do trait seems wrong since we
    // should never abort at this stage, which is why we used the
    // Execute trait directly. There is no risk in doing this action
    // more than once, but it's a bit wasteful. We should probably
    // introduce another trait which composes CheckMemory, Execute and
    // Remember to solve this problem (P.S. naming is hard)
    let herc20_redeem =
        Execute::<herc20::Redeemed>::execute(&bob, (herc20_deployed.clone(), hbit_redeemed.secret));
    let herc20_refund = Execute::<herc20::Refunded>::execute(&alice, herc20_deployed.clone());

    // It's always safe for Bob to redeem, he just has to do it before
    // Alice refunds
    match future::try_select(herc20_redeem, herc20_refund).await {
        Ok(future::Either::Left((_herc20_redeemed, _))) => Ok(()),
        Ok(future::Either::Right((_herc20_refunded, _))) => Ok(()),
        Err(either) => {
            let (error, _other_future) = either.factor_first();
            Err(error)
        }
    }
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
    ) -> (hbit::SharedParams, bitcoin::SecretKey, bitcoin::SecretKey) {
        let asset = asset::Bitcoin::from_sat(100_000_000);
        let expiry = Timestamp::now().plus(60 * 60);

        let (transient_refund_sk, transient_refund_pk) = {
            let transient_refund_sk = secp256k1::SecretKey::from_str(
                "01010101010101010001020304050607ffff0000ffff00006363636363636363",
            )
            .unwrap();
            let transient_refund_pk =
                identity::Bitcoin::from_secret_key(&crate::SECP, &transient_refund_sk);

            (transient_refund_sk, transient_refund_pk)
        };

        let (transient_redeem_sk, transient_redeem_pk) = {
            let transient_redeem_sk = secp256k1::SecretKey::from_str(
                "01010101010101010001020304050607ffff0000ffff00006363636363636363",
            )
            .unwrap();
            let transient_redeem_pk =
                identity::Bitcoin::from_secret_key(&crate::SECP, &transient_redeem_sk);

            (transient_redeem_sk, transient_redeem_pk)
        };

        let shared_params = hbit::SharedParams {
            network,
            asset,
            redeem_identity: transient_redeem_pk,
            refund_identity: transient_refund_pk,
            expiry,
            secret_hash,
        };

        (shared_params, transient_refund_sk, transient_redeem_sk)
    }

    fn secret() -> Secret {
        let bytes = b"hello world, you are beautiful!!";
        Secret::from(*bytes)
    }

    // TODO: Implement these traits on a real database
    #[derive(Clone, Copy)]
    struct Database;

    #[async_trait::async_trait]
    impl<E> db::Load<E> for Database
    where
        E: 'static,
    {
        async fn load(&self, _swap_id: SwapId) -> anyhow::Result<Option<E>> {
            Ok(None)
        }
    }

    #[async_trait::async_trait]
    impl<E> db::Save<E> for Database
    where
        E: Send + 'static,
    {
        async fn save(&self, _event: E, _swap_id: SwapId) -> anyhow::Result<()> {
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

        let (hbit_params, hbit_transient_refund_sk, hbit_transient_redeem_sk) =
            hbit_params(secret_hash, bitcoin_network);

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
                alpha_params: hbit::Params::new(hbit_params, hbit_transient_refund_sk),
                beta_params: herc20_params.clone(),
                secret,
                start_of_swap,
                swap_id,
            };
            let bob = WatchOnlyBob {
                alpha_connector: Arc::clone(&bitcoin_connector),
                beta_connector: Arc::clone(&ethereum_connector),
                db: alice_db,
                alpha_params: hbit_params,
                beta_params: herc20_params.clone(),
                secret_hash,
                start_of_swap,
                swap_id,
            };

            hbit_herc20(alice, bob)
        };

        let bob_swap = {
            let swap_id = SwapId::random();
            let alice = WatchOnlyAlice {
                alpha_connector: Arc::clone(&bitcoin_connector),
                beta_connector: Arc::clone(&ethereum_connector),
                db: bob_db,
                alpha_params: hbit_params,
                beta_params: herc20_params.clone(),
                secret_hash,
                start_of_swap,
                swap_id,
            };
            let bob = WalletBob {
                alpha_wallet: bob_bitcoin_wallet.clone(),
                beta_wallet: bob_ethereum_wallet.clone(),
                db: bob_db,
                alpha_params: hbit::Params::new(hbit_params, hbit_transient_redeem_sk),
                beta_params: herc20_params.clone(),
                secret_hash,
                start_of_swap,
                swap_id,
            };

            hbit_herc20(alice, bob)
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
