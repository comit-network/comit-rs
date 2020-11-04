//! Execute a swap.

pub mod bitcoin;
pub mod ethereum;
pub mod hbit;
pub mod herc20;

use crate::{
    command::FinishedSwap,
    database::{Load, Save},
    network::ActivePeer,
    SwapId,
};
use ::comit::btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector};
use anyhow::Result;
use comit::swap::Action;
use futures::{channel::mpsc, SinkExt, Stream, TryStreamExt};
use std::{future::Future, sync::Arc};
use time::OffsetDateTime;
use tracing_futures::Instrument;

pub use crate::database::Database;

#[derive(Clone, Debug, Eq, PartialEq, strum_macros::Display)]
pub enum SwapKind {
    HbitHerc20(SwapParams),
    Herc20Hbit(SwapParams),
}

impl SwapKind {
    pub fn params(&self) -> SwapParams {
        match self {
            SwapKind::HbitHerc20(params) | SwapKind::Herc20Hbit(params) => params.clone(),
        }
    }

    pub fn swap_id(&self) -> SwapId {
        self.params().swap_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapParams {
    pub hbit_params: hbit::Params,
    pub herc20_params: herc20::Params,
    pub secret_hash: comit::SecretHash,
    pub start_of_swap: OffsetDateTime,
    pub swap_id: SwapId,
    pub taker: ActivePeer,
}

#[cfg(test)]
impl crate::StaticStub for SwapParams {
    fn static_stub() -> Self {
        use ::bitcoin::secp256k1;
        use comit::SecretHash;
        use std::str::FromStr;

        let secret_hash =
            SecretHash::new(comit::Secret::from(*b"hello world, you are beautiful!!"));

        SwapParams {
            hbit_params: comit::swap::hbit::Params {
                shared: comit::hbit::SharedParams {
                    network: comit::ledger::Bitcoin::Regtest,
                    asset: comit::asset::Bitcoin::from_sat(12_345_678),
                    redeem_identity: comit::bitcoin::PublicKey::from_str(
                        "039b6347398505f5ec93826dc61c19f47c66c0283ee9be980e29ce325a0f4679ef",
                    )
                    .unwrap(),
                    refund_identity: comit::bitcoin::PublicKey::from_str(
                        "032e58afe51f9ed8ad3cc7897f634d881fdbe49a81564629ded8156bebd2ffd1af",
                    )
                    .unwrap(),
                    expiry: 12345678u32.into(),
                    secret_hash,
                },
                transient_sk: secp256k1::SecretKey::from_str(
                    "01010101010101010001020304050607ffff0000ffff00006363636363636363",
                )
                .unwrap(),
                final_address: "tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sl5k7"
                    .parse()
                    .unwrap(),
            },
            herc20_params: herc20::Params {
                asset: comit::asset::Erc20 {
                    token_contract: Default::default(),
                    quantity: comit::asset::Erc20Quantity::from_wei_dec_str(
                        "4_000_000_000_000_000_000",
                    )
                    .unwrap(),
                },
                redeem_identity: Default::default(),
                refund_identity: Default::default(),
                expiry: 987654321.into(),
                secret_hash,
                chain_id: 42.into(),
            },
            secret_hash: SecretHash::new(comit::Secret::from(*b"hello world, you are beautiful!!")),
            start_of_swap: OffsetDateTime::now_utc(),
            swap_id: Default::default(),
            taker: ActivePeer::static_stub(),
        }
    }
}

#[cfg(test)]
mod arbitrary {
    use super::*;
    use comit::SecretHash;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for SwapKind {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            if bool::arbitrary(g) {
                SwapKind::HbitHerc20(SwapParams::arbitrary(g))
            } else {
                SwapKind::Herc20Hbit(SwapParams::arbitrary(g))
            }
        }
    }

    impl Arbitrary for SwapParams {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SwapParams {
                hbit_params: hbit::Params::arbitrary(g),
                herc20_params: herc20::Params::arbitrary(g),
                secret_hash: SecretHash::arbitrary(g),
                start_of_swap: OffsetDateTime::from_unix_timestamp(u32::arbitrary(g) as i64),
                swap_id: SwapId::arbitrary(g),
                taker: ActivePeer::arbitrary(g),
            }
        }
    }
}

#[cfg(all(test, feature = "testcontainers"))]
mod tests {
    use super::*;
    use crate::{swap::bitcoin, test_harness, Seed, StaticStub, SwapId};
    use ::bitcoin::secp256k1;
    use comit::{
        asset::{
            self,
            ethereum::{Erc20Quantity, FromWei},
        },
        btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
        ethereum::ChainId,
        identity, ledger, Secret, SecretHash, Timestamp,
    };
    use std::{str::FromStr, sync::Arc};
    use testcontainers::clients;

    fn hbit_params(
        secret_hash: SecretHash,
        network: comit::ledger::Bitcoin,
    ) -> (
        comit::hbit::SharedParams,
        bitcoin::SecretKey,
        bitcoin::SecretKey,
    ) {
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

        let shared_params = comit::hbit::SharedParams {
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

    #[tokio::test]
    async fn execute_alice_hbit_herc20_swap() -> anyhow::Result<()> {
        let client = clients::Cli::default();

        let alice_db = Arc::new(Database::new_test().unwrap());
        let bob_db = Arc::new(Database::new_test().unwrap());

        let bitcoin_network = ledger::Bitcoin::Regtest;
        let (bitcoin_connector, bitcoind_url, bitcoin_blockchain) = {
            let blockchain = test_harness::bitcoin::Blockchain::new(&client)?;
            blockchain.init().await?;

            let node_url = blockchain.node_url.clone();

            (
                Arc::new(BitcoindConnector::new(node_url.clone())?),
                node_url,
                blockchain,
            )
        };
        let ethereum_chain_id = ethereum::ChainId::GETH_DEV;
        let (ethereum_connector, ethereum_node_url, ethereum_blockchain, token_contract) = {
            let mut blockchain = test_harness::ethereum::Blockchain::new(&client)?;
            blockchain.init().await?;

            let node_url = blockchain.node_url.clone();
            let token_contract = blockchain.token_contract();

            (
                Arc::new(Web3Connector::new(node_url.clone())),
                node_url,
                blockchain,
                token_contract,
            )
        };

        let (alice_bitcoin_wallet, alice_ethereum_wallet) = {
            let seed = Seed::random().unwrap();
            let bitcoin_wallet = {
                let wallet =
                    crate::bitcoin::Wallet::new(seed, bitcoind_url.clone(), bitcoin_network)
                        .await?;

                bitcoin_blockchain
                    .mint(
                        wallet.new_address().await?,
                        asset::Bitcoin::from_sat(1_000_000_000),
                    )
                    .await?;

                wallet
            };
            let ethereum_wallet = crate::ethereum::Wallet::new(
                seed,
                ethereum_node_url.clone(),
                crate::ethereum::Chain::new(ChainId::GETH_DEV, token_contract),
            )
            .await?;

            // mint ether to pay for gas
            ethereum_blockchain
                .mint_ether(
                    ethereum_wallet.account(),
                    1_000_000_000_000_000_000u64.into(),
                    ethereum_chain_id,
                )
                .await?;

            let ethereum_gas_price =
                crate::ethereum::GasPrice::geth_url(ethereum_blockchain.node_url.clone());

            let bitcoin_fee = crate::bitcoin::Fee::new(
                crate::config::Bitcoin {
                    network: ledger::Bitcoin::Regtest,
                    bitcoind: crate::config::Bitcoind {
                        node_url: bitcoind_url.clone(),
                    },
                    fees: crate::config::BitcoinFees::SatsPerByte(bitcoin::Amount::from_sat(50)),
                },
                crate::bitcoin::Client::new(bitcoind_url.clone()),
            );

            (
                bitcoin::Wallet {
                    inner: Arc::new(bitcoin_wallet),
                    connector: Arc::clone(&bitcoin_connector),
                    fee: bitcoin_fee,
                },
                ethereum::Wallet {
                    inner: Arc::new(ethereum_wallet),
                    connector: Arc::clone(&ethereum_connector),
                    gas_price: ethereum_gas_price,
                },
            )
        };

        let (bob_bitcoin_wallet, bob_ethereum_wallet) = {
            let seed = Seed::random().unwrap();
            let bitcoin_wallet =
                crate::bitcoin::Wallet::new(seed, bitcoind_url.clone(), bitcoin_network).await?;
            let ethereum_wallet = crate::ethereum::Wallet::new(
                seed,
                ethereum_node_url,
                crate::ethereum::Chain::new(ChainId::GETH_DEV, token_contract),
            )
            .await?;

            ethereum_blockchain
                .mint_erc20_token(
                    ethereum_wallet.account(),
                    asset::Erc20::new(token_contract, Erc20Quantity::from_wei(5_000_000_000u64)),
                    ethereum_chain_id,
                )
                .await?;

            // mint ether to pay for gas
            ethereum_blockchain
                .mint_ether(
                    ethereum_wallet.account(),
                    1_000_000_000_000_000_000u64.into(),
                    ethereum_chain_id,
                )
                .await?;

            let ethereum_gas_price =
                crate::ethereum::GasPrice::geth_url(ethereum_blockchain.node_url.clone());

            let bitcoin_fee = crate::bitcoin::Fee::new(
                crate::config::Bitcoin {
                    network: ledger::Bitcoin::Regtest,
                    bitcoind: crate::config::Bitcoind {
                        node_url: bitcoind_url.clone(),
                    },
                    fees: crate::config::BitcoinFees::SatsPerByte(bitcoin::Amount::from_sat(50)),
                },
                crate::bitcoin::Client::new(bitcoind_url.clone()),
            );

            (
                bitcoin::Wallet {
                    inner: Arc::new(bitcoin_wallet),
                    connector: Arc::clone(&bitcoin_connector),
                    fee: bitcoin_fee,
                },
                ethereum::Wallet {
                    inner: Arc::new(ethereum_wallet),
                    connector: Arc::clone(&ethereum_connector),
                    gas_price: ethereum_gas_price,
                },
            )
        };

        let secret = secret();
        let secret_hash = SecretHash::new(secret);

        let start_of_swap = OffsetDateTime::now_utc();
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
            let swap_id = SwapId::default();

            let hbit_params = hbit::Params {
                shared: hbit_params,
                transient_sk: hbit_transient_refund_sk,
                final_address: alice_bitcoin_wallet.inner.new_address().await?,
            };
            let swap = SwapKind::HbitHerc20(SwapParams {
                hbit_params: hbit_params.clone(),
                herc20_params: herc20_params.clone(),
                secret_hash,
                start_of_swap,
                swap_id,
                taker: ActivePeer::static_stub(),
            });

            alice_db.insert_swap(swap).await.unwrap();

            drive(
                comit::swap::hbit_herc20_alice(
                    hbit::Facade {
                        swap_id,
                        db: alice_db.clone(),
                        wallet: alice_bitcoin_wallet.clone(),
                    },
                    herc20::Facade {
                        swap_id,
                        db: alice_db.clone(),
                        wallet: alice_ethereum_wallet.clone(),
                    },
                    hbit_params,
                    herc20_params.clone(),
                    secret,
                    start_of_swap,
                ),
                alice_bitcoin_wallet.clone(),
                alice_ethereum_wallet.clone(),
                alice_db.clone(),
                swap_id,
            )
        };

        let bob_swap = {
            let swap_id = SwapId::default();

            let swap = SwapKind::HbitHerc20(SwapParams {
                hbit_params: hbit::Params {
                    shared: hbit_params,
                    transient_sk: hbit_transient_redeem_sk,
                    final_address: bob_bitcoin_wallet.inner.new_address().await?,
                },
                herc20_params: herc20_params.clone(),
                secret_hash,
                start_of_swap,
                swap_id,
                taker: ActivePeer::static_stub(),
            });

            bob_db.insert_swap(swap).await.unwrap();

            let hbit_params = hbit::Params {
                shared: hbit_params,
                transient_sk: hbit_transient_redeem_sk,
                final_address: bob_bitcoin_wallet.inner.new_address().await?,
            };

            drive(
                comit::swap::hbit_herc20_bob(
                    hbit::Facade {
                        swap_id,
                        db: bob_db.clone(),
                        wallet: bob_bitcoin_wallet.clone(),
                    },
                    herc20::Facade {
                        swap_id,
                        db: bob_db.clone(),
                        wallet: bob_ethereum_wallet.clone(),
                    },
                    hbit_params,
                    herc20_params.clone(),
                    start_of_swap,
                ),
                bob_bitcoin_wallet.clone(),
                bob_ethereum_wallet.clone(),
                bob_db.clone(),
                swap_id,
            )
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

        futures::future::try_join(alice_swap, bob_swap)
            .await
            .unwrap();

        // Sleep so that wallets have caught up with the balance changes caused by the
        // swap
        std::thread::sleep(std::time::Duration::from_millis(2000));

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
                >= alice_bitcoin_starting_balance - hbit_params.asset - bitcoin_max_fee
        );
        assert!(
            bob_bitcoin_final_balance
                >= bob_bitcoin_starting_balance + hbit_params.asset - bitcoin_max_fee
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

#[derive(Debug, Clone)]
pub struct SwapExecutor {
    db: Arc<Database>,
    bitcoin_wallet: Arc<crate::bitcoin::Wallet>,
    bitcoin_fee: crate::bitcoin::Fee,
    ethereum_wallet: Arc<crate::ethereum::Wallet>,
    ethereum_gas_price: crate::ethereum::GasPrice,
    finished_swap_sender: mpsc::Sender<FinishedSwap>,
    bitcoin_connector: Arc<BitcoindConnector>,
    ethereum_connector: Arc<Web3Connector>,
}

impl SwapExecutor {
    pub fn new(
        db: Arc<Database>,
        bitcoin_wallet: Arc<crate::bitcoin::Wallet>,
        bitcoin_fee: crate::bitcoin::Fee,
        ethereum_wallet: Arc<crate::ethereum::Wallet>,
        ethereum_gas_price: crate::ethereum::GasPrice,
        bitcoin_connector: Arc<BitcoindConnector>,
        ethereum_connector: Arc<Web3Connector>,
    ) -> (Self, mpsc::Receiver<FinishedSwap>) {
        // buffer increases by 1 for every clone of `Sender` and we use every sender
        // only once, hence making the initial buffer size 0 is good enough
        let buffer_size = 0;
        let (finished_swap_sender, finished_swap_receiver) = mpsc::channel(buffer_size);

        let executor = Self {
            db,
            bitcoin_wallet,
            bitcoin_fee,
            ethereum_wallet,
            ethereum_gas_price,
            finished_swap_sender,
            bitcoin_connector,
            ethereum_connector,
        };

        (executor, finished_swap_receiver)
    }
}

impl SwapExecutor {
    pub fn execute(&self, swap: SwapKind) {
        let execution = execute(
            swap.clone(),
            bitcoin::Wallet {
                inner: self.bitcoin_wallet.clone(),
                connector: self.bitcoin_connector.clone(),
                fee: self.bitcoin_fee.clone(),
            },
            ethereum::Wallet {
                inner: self.ethereum_wallet.clone(),
                connector: self.ethereum_connector.clone(),
                gas_price: self.ethereum_gas_price.clone(),
            },
            self.db.clone(),
            self.finished_swap_sender.clone(),
        );

        tokio::spawn(async move {
            if let Err(e) = execution.await {
                let err = e.context(format!("failed execution for swap {}", swap.swap_id()));

                sentry::integrations::anyhow::capture_anyhow(&err);
                tracing::warn!("{:#}", err);
            }
        });
    }
}

async fn execute(
    swap: SwapKind,
    bitcoin_wallet: bitcoin::Wallet,
    ethereum_wallet: ethereum::Wallet,
    db: Arc<Database>,
    mut sender: mpsc::Sender<FinishedSwap>,
) -> Result<()> {
    match swap.clone() {
        SwapKind::HbitHerc20(SwapParams {
            hbit_params,
            herc20_params,
            start_of_swap,
            swap_id,
            ..
        }) => {
            let swap = comit::swap::hbit_herc20_bob(
                hbit::Facade {
                    swap_id,
                    db: db.clone(),
                    wallet: bitcoin_wallet.clone(),
                },
                herc20::Facade {
                    swap_id,
                    db: db.clone(),
                    wallet: ethereum_wallet.clone(),
                },
                hbit_params,
                herc20_params,
                start_of_swap,
            )
            .instrument(tracing::error_span!("hbit_herc20_bob", %swap_id));

            drive(swap, bitcoin_wallet, ethereum_wallet, db, swap_id).await?;
        }
        SwapKind::Herc20Hbit(SwapParams {
            hbit_params,
            herc20_params,
            start_of_swap,
            swap_id,
            ..
        }) => {
            let swap = comit::swap::herc20_hbit_bob(
                herc20::Facade {
                    swap_id,
                    db: db.clone(),
                    wallet: ethereum_wallet.clone(),
                },
                hbit::Facade {
                    swap_id,
                    db: db.clone(),
                    wallet: bitcoin_wallet.clone(),
                },
                herc20_params,
                hbit_params,
                start_of_swap,
            )
            .instrument(tracing::error_span!("herc20_hbit_bob", %swap_id));

            drive(swap, bitcoin_wallet, ethereum_wallet, db, swap_id).await?;
        }
    };

    let active_peer = swap.params().taker;
    let swap_id = swap.swap_id();
    if let Err(e) = sender
        .send(FinishedSwap::new(
            swap,
            active_peer,
            OffsetDateTime::now_utc(),
        ))
        .await
    {
        tracing::warn!("failed to notify about finished swap {}", e)
    };

    tracing::info!("swap {} finished successfully", swap_id);

    sentry::capture_message(
        format!("successful execution for swap {}", swap_id).as_str(),
        sentry::Level::Info,
    );

    Ok(())
}

async fn drive<E>(
    mut swap: impl Stream<Item = Result<Action, E>> + Unpin,
    bitcoin_wallet: bitcoin::Wallet,
    ethereum_wallet: ethereum::Wallet,
    db: Arc<Database>,
    swap_id: SwapId,
) -> Result<()>
where
    E: std::error::Error + Send + Sync + 'static,
{
    while let Some(action) = swap.try_next().await? {
        match action {
            Action::Herc20Deploy(params) => {
                let action = ethereum_wallet.execute_deploy(params);

                execute_idempotently(db.as_ref(), swap_id, action).await?;
            }
            Action::Herc20Fund(params, deployed) => {
                let action = ethereum_wallet.execute_fund(params, deployed);

                execute_idempotently(db.as_ref(), swap_id, action).await?;
            }
            Action::Herc20Redeem(params, deployed, secret) => {
                let action = ethereum_wallet.execute_redeem(params, secret, deployed);

                execute_idempotently(db.as_ref(), swap_id, action).await?;
            }
            Action::HbitFund(params) => {
                let action = bitcoin_wallet.execute_fund(&params);

                execute_idempotently(db.as_ref(), swap_id, action).await?;
            }
            Action::HbitRedeem(params, funded, secret) => {
                let action = bitcoin_wallet.execute_redeem(params, funded, secret);

                execute_idempotently(db.as_ref(), swap_id, action).await?;
            }
        }
    }

    Ok(())
}

async fn execute_idempotently<T, DB>(
    db: &DB,
    swap_id: SwapId,
    action: impl Future<Output = Result<T>>,
) -> Result<()>
where
    DB: Load<T> + Save<T>,
    T: Clone + Send + Sync + 'static,
{
    if db.load(swap_id)?.is_some() {
        return Ok(());
    }

    let result = action.await?;
    db.save(result, swap_id).await?;

    Ok(())
}
