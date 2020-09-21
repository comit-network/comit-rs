//! Execute a swap.

mod action;
#[cfg(test)]
mod alice;
pub mod bitcoin;
mod bob;
mod comit;
pub mod ethereum;

use crate::{command::FinishedSwap, network::ActivePeer, swap::bob::Bob, SwapId};
use ::comit::btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use futures::{channel::mpsc, SinkExt};
use std::sync::Arc;
use tracing_futures::Instrument;

pub use self::comit::{hbit, herc20};
pub use crate::database::Database;

#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub start_of_swap: DateTime<Utc>,
    pub swap_id: SwapId,
    pub taker: ActivePeer,
}

/// Fetch the current network time for a ledger.
///
/// It returns a `anyhow::Result<comit::Timestamp>` so that it can be
/// used to determine whether a COMIT HTLC has expired.
#[async_trait::async_trait]
pub trait LedgerTime {
    async fn ledger_time(&self) -> anyhow::Result<comit::Timestamp>;
}

async fn poll_beta_has_expired<BC>(
    beta_connector: &BC,
    beta_expiry: comit::Timestamp,
) -> anyhow::Result<()>
where
    BC: LedgerTime,
{
    loop {
        let beta_ledger_time = beta_connector.ledger_time().await?;

        if beta_expiry <= beta_ledger_time {
            return Ok(());
        }

        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
impl crate::StaticStub for SwapParams {
    fn static_stub() -> Self {
        use crate::swap::hbit::SecretHash;
        use ::bitcoin::secp256k1;
        use std::str::FromStr;

        let secret_hash =
            SecretHash::new(comit::Secret::from(*b"hello world, you are beautiful!!"));

        SwapParams {
            hbit_params: hbit::Params {
                shared: hbit::SharedParams {
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
            start_of_swap: chrono::Utc::now(),
            swap_id: Default::default(),
            taker: ActivePeer::static_stub(),
        }
    }
}

#[cfg(test)]
mod arbitrary {
    use super::*;
    use crate::{
        arbitrary::*,
        swap::comit::{
            asset::{ethereum::TryFromWei, Erc20, Erc20Quantity},
            ethereum::ChainId,
        },
    };
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
            let herc20_params = herc20::Params {
                asset: erc20(g),
                redeem_identity: ethereum_address(g),
                refund_identity: ethereum_address(g),
                expiry: timestamp(g),
                secret_hash: secret_hash(g),
                chain_id: ChainId::from(u32::arbitrary(g)),
            };

            let naive = chrono::NaiveDateTime::from_timestamp(u32::arbitrary(g) as i64, 0);

            SwapParams {
                hbit_params: hbit::Params::arbitrary(g),
                herc20_params,
                secret_hash: secret_hash(g),
                start_of_swap: chrono::DateTime::from_utc(naive, chrono::offset::Utc),
                swap_id: SwapId::arbitrary(g),
                taker: ActivePeer::arbitrary(g),
            }
        }
    }

    fn ethereum_address<G: Gen>(g: &mut G) -> ethereum::Address {
        let mut bytes = [0u8; 20];
        for byte in &mut bytes {
            *byte = u8::arbitrary(g);
        }
        ethereum::Address::from(bytes)
    }

    fn erc20<G: Gen>(g: &mut G) -> Erc20 {
        let mut bytes = [0u8; 8];
        for byte in bytes.iter_mut() {
            *byte = u8::arbitrary(g);
        }
        let int = num::BigUint::from_bytes_be(&bytes);
        let quantity = Erc20Quantity::try_from_wei(int).unwrap();
        Erc20 {
            token_contract: ethereum_address(g),
            quantity,
        }
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::{
        swap::{
            alice::Alice,
            bitcoin,
            comit::{
                asset::{
                    self,
                    ethereum::{Erc20Quantity, FromWei},
                },
                btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
                ethereum::ChainId,
                identity, Secret, SecretHash, Timestamp,
            },
        },
        test_harness, Seed, StaticStub, SwapId,
    };
    use ::bitcoin::secp256k1;
    use chrono::Utc;
    use std::{str::FromStr, sync::Arc};
    use testcontainers::clients;

    fn hbit_params(
        secret_hash: SecretHash,
        network: comit::ledger::Bitcoin,
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

    #[tokio::test]
    async fn execute_alice_hbit_herc20_swap() -> anyhow::Result<()> {
        let client = clients::Cli::default();

        let alice_db = Arc::new(Database::new_test().unwrap());
        let bob_db = Arc::new(Database::new_test().unwrap());

        let bitcoin_network = ::bitcoin::Network::Regtest;
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

            (
                bitcoin::Wallet {
                    inner: Arc::new(bitcoin_wallet),
                    connector: Arc::clone(&bitcoin_connector),
                },
                ethereum::Wallet {
                    inner: Arc::new(ethereum_wallet),
                    connector: Arc::clone(&ethereum_connector),
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

            (
                bitcoin::Wallet {
                    inner: Arc::new(bitcoin_wallet),
                    connector: Arc::clone(&bitcoin_connector),
                },
                ethereum::Wallet {
                    inner: Arc::new(ethereum_wallet),
                    connector: Arc::clone(&ethereum_connector),
                },
            )
        };

        let secret = secret();
        let secret_hash = SecretHash::new(secret);

        let start_of_swap = Utc::now();
        let beta_expiry = Timestamp::now().plus(60 * 60);

        let (hbit_params, hbit_transient_refund_sk, hbit_transient_redeem_sk) =
            hbit_params(secret_hash, bitcoin_network.into());

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

            let swap = SwapKind::HbitHerc20(SwapParams {
                hbit_params: hbit::Params {
                    shared: hbit_params,
                    transient_sk: hbit_transient_refund_sk,
                },
                herc20_params: herc20_params.clone(),
                secret_hash,
                start_of_swap,
                swap_id,
                taker: ActivePeer::static_stub(),
            });

            alice_db.insert_swap(swap).await.unwrap();

            let hbit_params = hbit::Params::new(hbit_params, hbit_transient_refund_sk);
            let alice = Alice {
                alpha_wallet: alice_bitcoin_wallet.clone(),
                beta_wallet: alice_ethereum_wallet.clone(),
                db: Arc::clone(&alice_db),
                swap_id,
                secret,
                utc_start_of_swap: start_of_swap,
                beta_expiry: herc20_params.expiry,
            };

            comit::hbit_herc20_alice(
                alice,
                ethereum_connector.as_ref(),
                hbit_params,
                herc20_params.clone(),
                secret,
                start_of_swap,
            )
        };

        let bob_swap = {
            let swap_id = SwapId::default();

            let swap = SwapKind::HbitHerc20(SwapParams {
                hbit_params: hbit::Params {
                    shared: hbit_params,
                    transient_sk: hbit_transient_redeem_sk,
                },
                herc20_params: herc20_params.clone(),
                secret_hash,
                start_of_swap,
                swap_id,
                taker: ActivePeer::static_stub(),
            });

            bob_db.insert_swap(swap).await.unwrap();

            let hbit_params = hbit::Params::new(hbit_params, hbit_transient_redeem_sk);
            let bob = Bob {
                alpha_wallet: bob_bitcoin_wallet.clone(),
                beta_wallet: bob_ethereum_wallet.clone(),
                db: bob_db,
                swap_id,
                secret_hash,
                utc_start_of_swap: start_of_swap,
                beta_expiry: herc20_params.expiry,
            };

            comit::hbit_herc20_bob(
                bob,
                bitcoin_connector.as_ref(),
                ethereum_connector.as_ref(),
                hbit_params,
                herc20_params.clone(),
                start_of_swap,
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

        futures::future::try_join(alice_swap, bob_swap).await?;

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
    ethereum_wallet: Arc<crate::ethereum::Wallet>,
    finished_swap_sender: mpsc::Sender<FinishedSwap>,
    bitcoin_connector: Arc<BitcoindConnector>,
    ethereum_connector: Arc<Web3Connector>,
}

impl SwapExecutor {
    pub fn new(
        db: Arc<Database>,
        bitcoin_wallet: Arc<crate::bitcoin::Wallet>,
        ethereum_wallet: Arc<crate::ethereum::Wallet>,
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
            ethereum_wallet,
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
            swap,
            bitcoin::Wallet {
                inner: self.bitcoin_wallet.clone(),
                connector: self.bitcoin_connector.clone(),
            },
            self.bitcoin_connector.clone(),
            ethereum::Wallet {
                inner: self.ethereum_wallet.clone(),
                connector: self.ethereum_connector.clone(),
            },
            self.ethereum_connector.clone(),
            self.db.clone(),
            self.finished_swap_sender.clone(),
        );

        tokio::spawn(async move {
            if let Err(e) = execution.await {
                tracing::warn!("swap execution failed: {:#}", e);
            }
        });
    }
}

async fn execute(
    swap: SwapKind,
    bitcoin_wallet: bitcoin::Wallet,
    bitcoin_connector: Arc<BitcoindConnector>,
    ethereum_wallet: ethereum::Wallet,
    ethereum_connector: Arc<Web3Connector>,
    db: Arc<Database>,
    mut sender: mpsc::Sender<FinishedSwap>,
) -> Result<()> {
    match swap.clone() {
        SwapKind::HbitHerc20(SwapParams {
            hbit_params,
            herc20_params,
            secret_hash,
            start_of_swap,
            swap_id,
            ..
        }) => {
            let bob = Bob {
                alpha_wallet: bitcoin_wallet,
                beta_wallet: ethereum_wallet,
                db,
                swap_id,
                secret_hash,
                utc_start_of_swap: start_of_swap,
                beta_expiry: herc20_params.expiry,
            };

            comit::hbit_herc20_bob(
                bob,
                bitcoin_connector.as_ref(),
                ethereum_connector.as_ref(),
                hbit_params,
                herc20_params,
                start_of_swap,
            )
            .instrument(tracing::error_span!("hbit_herc20_bob", %swap_id))
            .await?
        }
        SwapKind::Herc20Hbit(SwapParams {
            hbit_params,
            herc20_params,
            secret_hash,
            start_of_swap,
            swap_id,
            ..
        }) => {
            let bob = Bob {
                alpha_wallet: ethereum_wallet,
                beta_wallet: bitcoin_wallet,
                db,
                swap_id,
                secret_hash,
                utc_start_of_swap: start_of_swap,
                beta_expiry: herc20_params.expiry,
            };

            comit::herc20_hbit_bob(
                bob,
                ethereum_connector.as_ref(),
                bitcoin_connector.as_ref(),
                herc20_params,
                hbit_params,
                start_of_swap,
            )
            .instrument(tracing::error_span!("herc20_hbit_bob", %swap_id))
            .await?
        }
    };

    let active_peer = swap.params().taker;
    let swap_id = swap.swap_id();
    sender
        .send(FinishedSwap::new(swap, active_peer, chrono::Utc::now()))
        .await
        .context("failed to notify about finished swap")?;

    tracing::info!("swap {} finished successfully", swap_id);

    Ok(())
}
