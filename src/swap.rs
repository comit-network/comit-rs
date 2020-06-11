use chrono::NaiveDateTime;
use comit::{
    actions,
    btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock},
    ethereum, SecretHash,
};

pub mod hbit {
    use bitcoin::secp256k1::Secp256k1;
    use bitcoin::*;
    use comit::{actions, identity, SecretHash};

    pub use cnd::{
        hbit::State,
        http_api::hbit::{FinalizedAsFunder, FinalizedAsRedeemer},
    };
    pub use comit::hbit::*;

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

    pub fn build_htlc_params_funder<C>(
        secp: &Secp256k1<C>,
        hbit: FinalizedAsFunder,
        secret_hash: SecretHash,
    ) -> Params
    where
        C: secp256k1::Signing,
    {
        let transient_refund_sk = hbit.transient_refund_identity;
        let refund_identity = identity::Bitcoin::from_secret_key(&secp, &transient_refund_sk);

        Params {
            network: hbit.network.into(),
            asset: hbit.asset,
            redeem_identity: hbit.transient_redeem_identity,
            refund_identity,
            expiry: hbit.expiry,
            secret_hash,
        }
    }

    pub fn build_htlc_params_redeemer<C>(
        secp: &Secp256k1<C>,
        hbit: FinalizedAsRedeemer,
        secret_hash: SecretHash,
    ) -> Params
    where
        C: secp256k1::Signing,
    {
        let transient_redeem_sk = hbit.transient_redeem_identity;
        let redeem_identity = identity::Bitcoin::from_secret_key(&secp, &transient_redeem_sk);

        Params {
            network: hbit.network.into(),
            asset: hbit.asset,
            redeem_identity,
            refund_identity: hbit.transient_refund_identity,
            expiry: hbit.expiry,
            secret_hash,
        }
    }
}

pub mod herc20 {
    use comit::SecretHash;

    pub use cnd::{herc20::State, http_api::herc20::Finalized};
    pub use comit::{actions::ethereum::*, herc20::*};

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

    pub fn build_htlc_params(herc20: Finalized, secret_hash: SecretHash) -> Params {
        Params {
            asset: herc20.asset,
            redeem_identity: herc20.redeem_identity,
            refund_identity: herc20.refund_identity,
            expiry: herc20.expiry,
            secret_hash,
            chain_id: herc20.chain_id,
        }
    }
}

pub struct Swap<A, B, R> {
    alpha: A,
    beta: B,
    role: R,
    created_at: NaiveDateTime,
}

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

impl Swap<hbit::FinalizedAsRedeemer, herc20::Finalized, Bob> {
    pub fn new(
        alpha_params: hbit::FinalizedAsRedeemer,
        beta_params: herc20::Finalized,
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

        let alpha_params = hbit::build_htlc_params_redeemer(&secp, self.alpha.clone(), secret_hash);
        let beta_params = herc20::build_htlc_params(self.beta.clone(), secret_hash);

        let _alpha_funded =
            hbit::watch_for_funded(&alpha_connector, &alpha_params, self.created_at).await?;

        let deploy_action = beta_params.build_deploy_action();
        let beta_deployed = beta_wallet.ensure_deploy(deploy_action).await?;

        let fund_action = beta_params.build_fund_action(beta_deployed.location)?;
        beta_wallet.ensure_fund(fund_action).await?;

        // TODO: Also be ready to refund as soon as it is possible.
        // Spawn away and use RUA feature of COMIT/cnd?

        let herc20::Redeemed { secret, .. } =
            herc20::watch_for_redeemed(&beta_connector, self.created_at, beta_deployed.clone())
                .await?;

        let redeem_action = self.alpha.build_redeem_action(secret)?;
        alpha_wallet.ensure_redeem(redeem_action).await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ledgers::{BitcoinBlockchain, EthereumBlockchain};
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

    fn hbit_finalized<C>(
        secp: &bitcoin::secp256k1::Secp256k1<C>,
    ) -> (hbit::FinalizedAsFunder, hbit::FinalizedAsRedeemer)
    where
        C: secp256k1::Signing,
    {
        let asset = asset::Bitcoin::from_sat(100_000_000);
        let network = Network::Regtest.into();

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
            bitcoin::Address::from_str("bcrt1q2nfxmhd4n3c8834pj72xagvyr9gl57n5r94fsl")
                .unwrap()
                .into();
        let finalized_as_funder = hbit::FinalizedAsFunder {
            asset,
            network,
            transient_redeem_identity: identity::Bitcoin::from_secret_key(
                &secp,
                &transient_redeem_sk,
            ),
            final_refund_identity,
            transient_refund_identity: transient_refund_sk,
            expiry: Timestamp::from(0),
            state: hbit::State::None,
        };

        // FIXME: Get final_redeem_identity from funder wallet
        let final_redeem_identity =
            bitcoin::Address::from_str("bcrt1q2nfxmhd4n3c8834pj72xagvyr9gl57n5r94fsl")
                .unwrap()
                .into();
        let finalized_as_redeemer = hbit::FinalizedAsRedeemer {
            asset,
            network,
            transient_redeem_identity: transient_redeem_sk,
            final_redeem_identity,
            transient_refund_identity: identity::Bitcoin::from_secret_key(
                &secp,
                &transient_refund_sk,
            ),
            expiry: Timestamp::from(0),
            state: hbit::State::None,
        };

        (finalized_as_funder, finalized_as_redeemer)
    }

    fn herc20_finalized() -> herc20::Finalized {
        let token_contract =
            ethereum::Address::from_str("6b175474e89094c44da98b954eedeac495271d0f").unwrap();
        let quantity = Erc20Quantity::from_wei(1_000u32);
        let asset = asset::Erc20::new(token_contract, quantity);

        // FIXME: Obtain identities from user wallets
        let identity =
            identity::Ethereum::from_str("c5549e335b2786520f4c5d706c76c9ee69d0a028").unwrap();

        herc20::Finalized {
            asset,
            chain_id: ethereum::ChainId::regtest(),
            refund_identity: identity,
            redeem_identity: identity,
            expiry: Timestamp::from(0),
            state: herc20::State::None,
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
        ) -> anyhow::Result<cnd::hbit::Redeemed> {
            todo!()
        }
    }

    struct MockEthereumWallet;

    #[async_trait::async_trait]
    impl herc20::EnsureDeploy for MockEthereumWallet {
        async fn ensure_deploy(
            &self,
            _action: comit::actions::ethereum::DeployContract,
        ) -> anyhow::Result<cnd::herc20::Deployed> {
            todo!()
        }
    }

    #[async_trait::async_trait]
    impl herc20::EnsureFund for MockEthereumWallet {
        async fn ensure_fund(
            &self,
            _action: comit::actions::ethereum::CallContract,
        ) -> anyhow::Result<cnd::herc20::Funded> {
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
    impl Swap<hbit::FinalizedAsFunder, herc20::Finalized, Alice> {
        pub fn new(
            alpha_params: hbit::FinalizedAsFunder,
            beta_params: herc20::Finalized,
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
            _secp: &bitcoin::secp256k1::Secp256k1<C>,
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

            let fund_action = self.alpha.build_fund_action(secret_hash);
            alpha_wallet.ensure_fund(fund_action).await?;

            let beta_params = herc20::build_htlc_params(self.beta.clone(), secret_hash);
            let beta_deployed =
                herc20::watch_for_deployed(&beta_connector, beta_params.clone(), self.created_at)
                    .await?;
            let _beta_funded = herc20::watch_for_funded(
                &beta_connector,
                beta_params.clone(),
                self.created_at,
                beta_deployed,
            )
            .await?;

            let redeem_action = self.beta.build_redeem_action(self.role.secret)?;
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
        let (hbit_finalized_as_funder, hbit_finalized_as_redeemer) = hbit_finalized(&secp);
        let alice_swap = Swap::<_, _, Alice>::new(
            hbit_finalized_as_funder,
            herc20_finalized(),
            alice,
            Utc::now().naive_local(),
        );

        let bob = Bob { secret_hash };
        let bob_swap = Swap::<_, _, Bob>::new(
            hbit_finalized_as_redeemer,
            herc20_finalized(),
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
