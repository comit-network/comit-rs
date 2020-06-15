use chrono::NaiveDateTime;
use comit::{
    actions,
    btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock},
    ethereum, SecretHash,
};

pub mod hbit {
    use bitcoin::secp256k1::{Secp256k1, SecretKey};
    use bitcoin::*;
    use chrono::NaiveDateTime;
    use comit::{actions, asset, identity, SecretHash, Timestamp};

    pub use comit::{
        btsieve::{BlockByHash, LatestBlock},
        hbit::*,
        htlc_location, transaction,
    };

    // TODO: Find a better name
    #[derive(Clone, Debug)]
    pub struct SwapDetailsFunder {
        pub network: bitcoin::Network,
        pub asset: asset::Bitcoin,
        pub transient_redeem_identity: identity::Bitcoin,
        pub transient_refund_identity: SecretKey,
        pub final_refund_identity: Address,
        pub expiry: Timestamp,
    }

    // TODO: Find a better name
    #[derive(Clone, Debug)]
    pub struct SwapDetailsRedeemer {
        pub network: bitcoin::Network,
        pub asset: asset::Bitcoin,
        pub transient_redeem_identity: SecretKey,
        pub transient_refund_identity: identity::Bitcoin,
        pub final_redeem_identity: Address,
        pub expiry: Timestamp,
    }

    impl SwapDetailsFunder {
        pub fn build_htlc_params_funder<C>(
            &self,
            secp: &Secp256k1<C>,
            secret_hash: SecretHash,
        ) -> Params
        where
            C: secp256k1::Signing,
        {
            let transient_refund_sk = self.transient_refund_identity;
            let refund_identity = identity::Bitcoin::from_secret_key(&secp, &transient_refund_sk);

            Params {
                network: self.network,
                asset: self.asset,
                redeem_identity: self.transient_redeem_identity,
                refund_identity,
                expiry: self.expiry,
                secret_hash,
            }
        }
    }

    impl SwapDetailsRedeemer {
        pub fn build_htlc_params_redeemer<C>(
            &self,
            secp: &Secp256k1<C>,
            secret_hash: SecretHash,
        ) -> Params
        where
            C: secp256k1::Signing,
        {
            let transient_redeem_sk = self.transient_redeem_identity;
            let redeem_identity = identity::Bitcoin::from_secret_key(&secp, &transient_redeem_sk);

            Params {
                network: self.network,
                asset: self.asset,
                redeem_identity,
                refund_identity: self.transient_refund_identity,
                expiry: self.expiry,
                secret_hash,
            }
        }
    }

    #[async_trait::async_trait]
    pub trait EnsureFund {
        async fn ensure_fund(&self, action: actions::bitcoin::SendToAddress) -> anyhow::Result<()>;
    }

    #[async_trait::async_trait]
    pub trait EnsureRedeem {
        async fn ensure_redeem(
            &self,
            action: actions::bitcoin::BroadcastSignedTransaction,
        ) -> anyhow::Result<Redeemed>;
    }

    #[async_trait::async_trait]
    pub trait EnsureRefund {
        async fn ensure_fund(
            &self,
            action: actions::bitcoin::BroadcastSignedTransaction,
        ) -> anyhow::Result<Refunded>;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct CorrectlyFunded {
        pub asset: asset::Bitcoin,
        pub location: htlc_location::Bitcoin,
    }

    pub async fn watch_for_funded<C>(
        connector: &C,
        params: &Params,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<CorrectlyFunded>
    where
        C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = BlockHash>,
    {
        match comit::hbit::watch_for_funded(connector, params, start_of_swap).await? {
            Funded::Correctly {
                asset, location, ..
            } => Ok(CorrectlyFunded { asset, location }),
            Funded::Incorrectly { .. } => anyhow::bail!("Bitcoin HTLC incorrectly funded"),
        }
    }
}
pub mod herc20 {
    use chrono::NaiveDateTime;
    pub use comit::{
        actions::ethereum::*,
        asset,
        btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock},
        ethereum::ChainId,
        ethereum::{Block, Hash},
        herc20::*,
        identity, transaction, SecretHash, Timestamp,
    };

    #[derive(Debug, Clone)]
    pub struct SwapDetails {
        pub asset: asset::Erc20,
        pub redeem_identity: identity::Ethereum,
        pub refund_identity: identity::Ethereum,
        pub expiry: Timestamp,
        pub chain_id: ChainId,
    }

    impl SwapDetails {
        pub fn build_htlc_params(&self, secret_hash: SecretHash) -> Params {
            Params {
                asset: self.asset.clone(),
                redeem_identity: self.redeem_identity,
                refund_identity: self.refund_identity,
                expiry: self.expiry,
                secret_hash,
                chain_id: self.chain_id,
            }
        }
    }

    #[async_trait::async_trait]
    pub trait EnsureDeploy {
        async fn ensure_deploy(&self, action: DeployContract) -> anyhow::Result<Deployed>;
    }

    #[async_trait::async_trait]
    pub trait EnsureFund {
        async fn ensure_fund(&self, action: CallContract) -> anyhow::Result<Funded>;
    }

    #[async_trait::async_trait]
    pub trait EnsureRedeem {
        async fn ensure_redeem(&self, action: CallContract) -> anyhow::Result<Redeemed>;
    }

    #[async_trait::async_trait]
    pub trait EnsureRefund {
        async fn ensure_fund(&self, action: CallContract) -> anyhow::Result<Refunded>;
    }

    #[derive(Debug, Clone)]
    pub struct CorrectlyFunded {
        pub transaction: transaction::Ethereum,
        pub asset: asset::Erc20,
    }

    pub async fn watch_for_funded<C>(
        connector: &C,
        params: Params,
        start_of_swap: NaiveDateTime,
        deployed: Deployed,
    ) -> anyhow::Result<CorrectlyFunded>
    where
        C: LatestBlock<Block = Block>
            + BlockByHash<Block = Block, BlockHash = Hash>
            + ReceiptByHash,
    {
        match comit::herc20::watch_for_funded(connector, params, start_of_swap, deployed).await? {
            comit::herc20::Funded::Correctly { transaction, asset } => {
                Ok(CorrectlyFunded { transaction, asset })
            }
            comit::herc20::Funded::Incorrectly { .. } => {
                anyhow::bail!("Ethereum HTLC incorrectly funded")
            }
        }
    }
}

#[derive(Debug)]
pub struct Swap<A, B, R> {
    alpha: A,
    beta: B,
    role: R,
    created_at: NaiveDateTime,
}

#[derive(Clone, Copy, Debug)]
pub struct Bob {
    secret_hash: SecretHash,
}

pub trait SendToAddress {
    fn send_to_address(&self, action: actions::bitcoin::SendToAddress) -> anyhow::Result<()>;
}

pub trait BroadcastSignedTransaction {
    fn send_to_address(
        &self,
        action: actions::bitcoin::BroadcastSignedTransaction,
    ) -> anyhow::Result<()>;
}

pub trait CallContract {
    fn call_contract(&self, action: actions::ethereum::CallContract) -> anyhow::Result<()>;
}

impl Swap<hbit::SwapDetailsRedeemer, herc20::SwapDetails, Bob> {
    pub fn new(
        alpha_params: hbit::SwapDetailsRedeemer,
        beta_params: herc20::SwapDetails,
        bob: Bob,
        created_at: NaiveDateTime,
    ) -> Self {
        Self {
            alpha: alpha_params,
            beta: beta_params,
            role: bob,
            created_at,
        }
    }

    pub async fn execute<C, AC, BC, AW, BW>(
        &self,
        secp: &bitcoin::secp256k1::Secp256k1<C>,
        alpha_connector: AC,
        beta_connector: BC,
        alpha_wallet: AW,
        beta_wallet: BW,
    ) -> anyhow::Result<()>
    where
        C: bitcoin::secp256k1::Signing,
        AC: LatestBlock<Block = bitcoin::Block>
            + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        AW: hbit::EnsureRedeem,
        BW: herc20::EnsureDeploy + herc20::EnsureFund,
    {
        // QUESTION: If we are respawning, how do we choose between
        // watching for an action tha we may have performed in the
        // past or executing it? If we watch for it, when do we stop
        // looking for it and assume that we haven't done it yet?
        //
        // MY ANSWER: The wallet can either check by itself or
        // delegate to btsieve under the hood e.g. it's hard to watch
        // for Bitcoin's sendtoaddress with just a bitcoind wallet.
        // This could be achieved through the `Ensure<Action>` traits

        let secret_hash = self.role.secret_hash;

        let alpha_params = self.alpha.build_htlc_params_redeemer(&secp, secret_hash);
        let beta_params = self.beta.build_htlc_params(secret_hash);

        let hbit::CorrectlyFunded {
            asset: fund_amount,
            location: fund_location,
            ..
        } = hbit::watch_for_funded(&alpha_connector, &alpha_params, self.created_at).await?;

        let deploy_action = beta_params.build_deploy_action();
        let beta_deployed = beta_wallet.ensure_deploy(deploy_action).await?;

        let fund_action = beta_params.build_fund_action(beta_deployed.location)?;
        beta_wallet.ensure_fund(fund_action).await?;

        // TODO: Also be ready to refund as soon as it is possible.
        // Spawn away and use RUA feature of COMIT/cnd?

        let herc20::Redeemed { secret, .. } =
            herc20::watch_for_redeemed(&beta_connector, self.created_at, beta_deployed.clone())
                .await?;

        let redeem_action = alpha_params.build_redeem_action(
            &secp,
            fund_amount,
            fund_location,
            self.alpha.transient_redeem_identity,
            self.alpha.final_redeem_identity.clone(),
            secret,
        )?;
        alpha_wallet.ensure_redeem(redeem_action).await?;

        Ok(())
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::test_harness::{BitcoinBlockchain, EthereumBlockchain};
    use bitcoin::{secp256k1, Network};
    use chrono::Utc;
    use comit::{
        asset::{
            self,
            ethereum::{Erc20Quantity, FromWei},
        },
        btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
        ethereum, identity, Secret, SecretHash, Timestamp,
    };
    use std::str::FromStr;
    use testcontainers::clients;

    fn hbit_swap_details<C>(
        secp: &bitcoin::secp256k1::Secp256k1<C>,
    ) -> (hbit::SwapDetailsFunder, hbit::SwapDetailsRedeemer)
    where
        C: secp256k1::Signing,
    {
        let asset = asset::Bitcoin::from_sat(100_000_000);
        let network = Network::Regtest;

        let transient_refund_sk = secp256k1::SecretKey::from_str(
            "01010101010101010001020304050607ffff0000ffff00006363636363636363",
        )
        .unwrap();
        let transient_redeem_sk = secp256k1::SecretKey::from_str(
            "01010101010101010001020304050607ffff0000ffff00006363636363636363",
        )
        .unwrap();

        // FIXME: Get final_refund_identity from funder wallet
        let final_refund_identity =
            bitcoin::Address::from_str("bcrt1q2nfxmhd4n3c8834pj72xagvyr9gl57n5r94fsl").unwrap();
        let finalized_as_funder = hbit::SwapDetailsFunder {
            network,
            asset,
            transient_redeem_identity: identity::Bitcoin::from_secret_key(
                &secp,
                &transient_redeem_sk,
            ),
            transient_refund_identity: transient_refund_sk,
            final_refund_identity,
            expiry: Timestamp::from(0),
        };

        // FIXME: Get final_redeem_identity from funder wallet
        let final_redeem_identity =
            bitcoin::Address::from_str("bcrt1q2nfxmhd4n3c8834pj72xagvyr9gl57n5r94fsl").unwrap();
        let finalized_as_redeemer = hbit::SwapDetailsRedeemer {
            network,
            asset,
            transient_redeem_identity: transient_redeem_sk,
            transient_refund_identity: identity::Bitcoin::from_secret_key(
                &secp,
                &transient_refund_sk,
            ),
            final_redeem_identity,
            expiry: Timestamp::from(0),
        };

        (finalized_as_funder, finalized_as_redeemer)
    }

    fn herc20_swap_details() -> herc20::SwapDetails {
        let token_contract =
            ethereum::Address::from_str("6b175474e89094c44da98b954eedeac495271d0f").unwrap();
        let quantity = Erc20Quantity::from_wei(1_000u32);
        let asset = asset::Erc20::new(token_contract, quantity);

        // FIXME: Obtain identities from user wallets
        let identity =
            identity::Ethereum::from_str("c5549e335b2786520f4c5d706c76c9ee69d0a028").unwrap();

        herc20::SwapDetails {
            asset,
            redeem_identity: identity,
            refund_identity: identity,
            expiry: Timestamp::from(0),
            chain_id: ethereum::ChainId::regtest(),
        }
    }

    struct MockBitcoinWallet;

    #[async_trait::async_trait]
    impl hbit::EnsureFund for MockBitcoinWallet {
        async fn ensure_fund(
            &self,
            _action: comit::actions::bitcoin::SendToAddress,
        ) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[async_trait::async_trait]
    impl hbit::EnsureRedeem for MockBitcoinWallet {
        async fn ensure_redeem(
            &self,
            _action: comit::actions::bitcoin::BroadcastSignedTransaction,
        ) -> anyhow::Result<hbit::Redeemed> {
            todo!()
        }
    }

    struct MockEthereumWallet;

    #[async_trait::async_trait]
    impl herc20::EnsureDeploy for MockEthereumWallet {
        async fn ensure_deploy(
            &self,
            _action: comit::actions::ethereum::DeployContract,
        ) -> anyhow::Result<herc20::Deployed> {
            todo!()
        }
    }

    #[async_trait::async_trait]
    impl herc20::EnsureFund for MockEthereumWallet {
        async fn ensure_fund(
            &self,
            _action: comit::actions::ethereum::CallContract,
        ) -> anyhow::Result<herc20::Funded> {
            todo!()
        }
    }

    #[async_trait::async_trait]
    impl herc20::EnsureRedeem for MockEthereumWallet {
        async fn ensure_redeem(
            &self,
            _action: comit::actions::ethereum::CallContract,
        ) -> anyhow::Result<herc20::Redeemed> {
            todo!()
        }
    }

    fn secret() -> Secret {
        let bytes = b"hello world, you are beautiful!!";
        Secret::from(*bytes)
    }

    struct Alice {
        secret: Secret,
    }

    // Interestingly, Alice doesn't care about what Bob does on alpha
    // ledger. We therefore don't pass the `alpha_connector`.
    impl Swap<hbit::SwapDetailsFunder, herc20::SwapDetails, Alice> {
        pub fn new(
            alpha_params: hbit::SwapDetailsFunder,
            beta_params: herc20::SwapDetails,
            alice: Alice,
            created_at: NaiveDateTime,
        ) -> Self {
            Self {
                alpha: alpha_params,
                beta: beta_params,
                role: alice,
                created_at,
            }
        }

        async fn execute<C, BC, AW, BW>(
            &self,
            secp: &bitcoin::secp256k1::Secp256k1<C>,
            beta_connector: BC,
            alpha_wallet: AW,
            beta_wallet: BW,
        ) -> anyhow::Result<()>
        where
            C: bitcoin::secp256k1::Signing,
            BC: LatestBlock<Block = ethereum::Block>
                + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
                + ReceiptByHash,
            AW: hbit::EnsureFund,
            BW: herc20::EnsureRedeem,
        {
            let secret = self.role.secret;
            let secret_hash = SecretHash::new(secret);

            let alpha_params = self.alpha.build_htlc_params_funder(&secp, secret_hash);
            let beta_params = self.beta.build_htlc_params(secret_hash);

            let fund_action = alpha_params.build_fund_action();
            alpha_wallet.ensure_fund(fund_action).await?;

            let beta_deployed =
                herc20::watch_for_deployed(&beta_connector, beta_params.clone(), self.created_at)
                    .await?;
            let _beta_funded = herc20::watch_for_funded(
                &beta_connector,
                beta_params.clone(),
                self.created_at,
                beta_deployed.clone(),
            )
            .await?;

            let redeem_action =
                beta_params.build_redeem_action(beta_deployed.location, self.role.secret)?;
            beta_wallet.ensure_redeem(redeem_action).await?;

            // NOTE: I don't think we care about the rest of the swap, so long
            // as we redeemed and we're actively waiting to perform refund
            // if it becomes available. Therefore, we can exit.

            Ok(())
        }
    }

    // The nectar application should only ever care about one side of
    // the swap, but for the purposes of testing the `execute`
    // interface it is convenient to drive the other side forward
    // using that same interface
    #[tokio::test]
    async fn execute_alice_hbit_herc20_swap() {
        let secp: bitcoin::secp256k1::Secp256k1<bitcoin::secp256k1::All> =
            bitcoin::secp256k1::Secp256k1::new();

        let (_alice_bitcoin_connector, bob_bitcoin_connector) = {
            let client = clients::Cli::default();
            let blockchain = BitcoinBlockchain::new(&client).unwrap();
            (
                BitcoindConnector::new(blockchain.node_url.clone(), Network::Regtest).unwrap(),
                BitcoindConnector::new(blockchain.node_url, Network::Regtest).unwrap(),
            )
        };
        let (alice_ethereum_connector, bob_ethereum_connector) = {
            let client = clients::Cli::default();
            let blockchain = EthereumBlockchain::new(&client).unwrap();
            (
                Web3Connector::new(blockchain.node_url.clone()),
                Web3Connector::new(blockchain.node_url),
            )
        };

        let secret = secret();
        let alice = Alice { secret };
        let secret_hash = SecretHash::new(secret);
        let (hbit_finalized_as_funder, hbit_finalized_as_redeemer) = hbit_swap_details(&secp);
        let alice_swap = Swap::<_, _, Alice>::new(
            hbit_finalized_as_funder,
            herc20_swap_details(),
            alice,
            Utc::now().naive_local(),
        );

        let bob = Bob { secret_hash };
        let bob_swap = Swap::<_, _, Bob>::new(
            hbit_finalized_as_redeemer,
            herc20_swap_details(),
            bob,
            Utc::now().naive_local(),
        );

        // Wallets implement interface defined by _us_

        let alice_bitcoin_wallet = MockBitcoinWallet;
        let alice_ethereum_wallet = MockEthereumWallet;
        let _ = alice_swap.execute(
            &secp,
            alice_ethereum_connector,
            alice_bitcoin_wallet,
            alice_ethereum_wallet,
        );

        let bob_bitcoin_wallet = MockBitcoinWallet;
        let bob_ethereum_wallet = MockEthereumWallet;
        let _ = bob_swap.execute(
            &secp,
            bob_bitcoin_connector,
            bob_ethereum_connector,
            bob_bitcoin_wallet,
            bob_ethereum_wallet,
        );

        // TODO: Actually spawn both swap executions
        // TODO: Assert that the money moves as expected
    }
}
