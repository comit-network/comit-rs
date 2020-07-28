use crate::ethereum::DAI_TRANSFER_GAS_LIMIT;
use crate::{
    ethereum::{
        dai, ether,
        geth::{Client, EstimateGasRequest},
        Address, ChainId, Hash,
    },
    Seed,
};
use comit::{
    actions::ethereum::{CallContract, DeployContract},
    asset::Erc20,
    ethereum::{Transaction, TransactionReceipt},
};
use num::BigUint;
use num256::Uint256;
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone)]
pub struct Wallet {
    private_key: clarity::PrivateKey,
    geth_client: Client,
    dai_contract_addr: Address,
    pub chain_id: ChainId,
}

impl Wallet {
    pub async fn new(
        seed: Seed,
        url: Url,
        dai_contract_addr: Address,
        chain_id: ChainId,
    ) -> anyhow::Result<Self> {
        let private_key = Wallet::private_key_from_seed(&seed)?;

        let geth_client = Client::new(url);

        let wallet = Self {
            private_key,
            geth_client,
            dai_contract_addr,
            chain_id,
        };

        wallet.assert_chain(chain_id).await?;

        Ok(wallet)
    }

    pub fn private_key_from_seed(seed: &Seed) -> anyhow::Result<clarity::PrivateKey> {
        let private_key = clarity::PrivateKey::from_slice(&seed.bytes())
            .map_err(|_| anyhow::anyhow!("Failed to derive private key from slice"))?;
        Ok(private_key)
    }

    #[cfg(test)]
    pub fn new_from_private_key(
        private_key: clarity::PrivateKey,
        url: Url,
        chain_id: ChainId,
    ) -> Self {
        let geth_client = Client::new(url);
        let dai_contract_adr = Address::random();

        Self {
            private_key,
            geth_client,
            dai_contract_addr: dai_contract_adr,
            chain_id,
        }
    }

    pub fn account(&self) -> Address {
        let pk = self.private_key.to_public_key().expect("cannot fail");

        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(pk.as_bytes());

        Address::from(bytes)
    }

    pub fn private_key(&self) -> clarity::PrivateKey {
        self.private_key
    }

    pub async fn deploy_contract(
        &self,
        DeployContract {
            data,
            gas_limit,
            chain_id,
            ..
        }: DeployContract,
    ) -> anyhow::Result<DeployedContract> {
        self.assert_chain(chain_id).await?;

        let nonce = self.get_transaction_count().await?;
        let gas_price = self.gas_price().await?;

        let transaction = clarity::Transaction {
            nonce: nonce.into(),
            gas_price,
            gas_limit: gas_limit.into(),
            to: clarity::Address::default(),
            value: 0u64.into(),
            data,
            signature: None,
        };

        let signed_transaction =
            transaction.sign(&self.private_key, Some(u32::from(chain_id) as u64));
        let transaction_hex =
            format!(
                "0x{}",
                hex::encode(signed_transaction.to_bytes().map_err(|_| anyhow::anyhow!(
                    "Failed to serialize signed transaction to bytes"
                ))?)
            );

        let hash = self
            .geth_client
            .send_raw_transaction(transaction_hex)
            .await?;

        let contract_address = match self.wait_until_transaction_receipt(hash).await? {
            TransactionReceipt {
                status: 1,
                contract_address: Some(contract_address),
                ..
            } => contract_address,
            TransactionReceipt { status: 0, .. } => {
                anyhow::bail!("Transaction receipt status failed")
            }
            TransactionReceipt {
                contract_address: None,
                ..
            } => anyhow::bail!("No contract address in deployment transaction receipt"),
            receipt => anyhow::bail!("Fetched invalid transaction receipt: {:?}", receipt),
        };

        let transaction = self.get_transaction_by_hash(hash).await?;

        Ok(DeployedContract {
            transaction,
            contract_address,
        })
    }

    pub async fn send_transaction(
        &self,
        to: Address,
        value: ether::Amount,
        gas_limit: Option<u64>,
        data: Option<Vec<u8>>,
        chain_id: ChainId,
    ) -> anyhow::Result<Hash> {
        self.assert_chain(chain_id).await?;

        let nonce = self.get_transaction_count().await?;
        let gas_price = self.gas_price().await?;

        let gas_limit = match gas_limit {
            Some(gas_limit) => gas_limit.into(),
            None => {
                self.gas_limit(EstimateGasRequest {
                    from: None,
                    to: Some(to),
                    gas_price: Some(gas_price.clone()),
                    value: Some(value.clone().into()),
                    data: data.clone(),
                })
                .await?
            }
        };

        let to = clarity::Address::from_slice(to.as_bytes())
            .map_err(|_| anyhow::anyhow!("Failed to deserialize slice into clarity::Address"))?;

        let transaction = clarity::Transaction {
            nonce: nonce.into(),
            gas_price,
            gas_limit,
            to,
            value: value.into(),
            data: data.unwrap_or_default(),
            signature: None,
        };

        #[allow(clippy::cast_possible_truncation)]
        let signed_transaction =
            transaction.sign(&self.private_key, Some(u32::from(chain_id) as u64));
        let transaction_hex =
            format!(
                "0x{}",
                hex::encode(signed_transaction.to_bytes().map_err(|_| anyhow::anyhow!(
                    "Failed to serialize signed transaction to bytes"
                ))?)
            );

        let hash = self
            .geth_client
            .send_raw_transaction(transaction_hex)
            .await?;

        // TODO: Return TransactionReceipt instead
        std::thread::sleep(std::time::Duration::from_millis(2000));

        Ok(hash)
    }

    pub async fn transfer_dai(
        &self,
        to: Address,
        value: dai::Amount,
        chain_id: ChainId,
    ) -> anyhow::Result<Hash> {
        self.assert_chain(chain_id).await?;

        let nonce = self.get_transaction_count().await?;
        let gas_price = self.gas_price().await?;

        let to = clarity::Address::from_slice(to.as_bytes())
            .map_err(|_| anyhow::anyhow!("Failed to deserialize slice into clarity::Address"))?;

        let dai_contract_addr = clarity::Address::from_slice(self.dai_contract_addr.as_bytes())
            .map_err(|_| anyhow::anyhow!("Failed to deserialize slice into clarity::Address"))?;

        let data = clarity::abi::encode_call(
            "transfer(address,uint256)",
            &[
                clarity::abi::Token::Address(to),
                clarity::abi::Token::Uint(Uint256::from_bytes_le(value.to_bytes().as_slice())),
            ],
        );

        let transaction = clarity::Transaction {
            nonce: nonce.into(),
            gas_price,
            gas_limit: DAI_TRANSFER_GAS_LIMIT.into(),
            to: dai_contract_addr,
            value: 0u16.into(),
            data,
            signature: None,
        };

        #[allow(clippy::cast_possible_truncation)]
        let signed_transaction =
            transaction.sign(&self.private_key, Some(u32::from(chain_id) as u64));
        let transaction_hex =
            format!(
                "0x{}",
                hex::encode(signed_transaction.to_bytes().map_err(|_| anyhow::anyhow!(
                    "Failed to serialize signed transaction to bytes"
                ))?)
            );

        let hash = self
            .geth_client
            .send_raw_transaction(transaction_hex)
            .await?;

        // TODO: Return TransactionReceipt instead
        std::thread::sleep(std::time::Duration::from_millis(2000));

        Ok(hash)
    }

    pub async fn call_contract(
        &self,
        CallContract {
            to,
            data,
            gas_limit,
            chain_id,
            ..
        }: CallContract,
    ) -> anyhow::Result<Hash> {
        self.assert_chain(chain_id).await?;

        let nonce = self.get_transaction_count().await?;
        let gas_price = self.gas_price().await?;

        let transaction = clarity::Transaction {
            nonce: nonce.into(),
            gas_price,
            gas_limit: gas_limit.into(),
            to: clarity::Address::from_slice(to.as_bytes()).map_err(|_| {
                anyhow::anyhow!("Failed to deserialize slice into clarity::Address")
            })?,
            value: 0u32.into(),
            data: data.unwrap_or_default(),
            signature: None,
        };

        #[allow(clippy::cast_possible_truncation)]
        let signed_transaction =
            transaction.sign(&self.private_key, Some(u32::from(chain_id) as u64));
        let transaction_hex =
            format!(
                "0x{}",
                hex::encode(signed_transaction.to_bytes().map_err(|_| anyhow::anyhow!(
                    "Failed to serialize signed transaction to bytes"
                ))?)
            );

        let hash = self
            .geth_client
            .send_raw_transaction(transaction_hex)
            .await?;

        // TODO: Return TransactionReceipt instead
        std::thread::sleep(std::time::Duration::from_millis(2000));

        Ok(hash)
    }

    pub async fn get_transaction_by_hash(
        &self,
        transaction_hash: Hash,
    ) -> anyhow::Result<Transaction> {
        self.geth_client
            .get_transaction_by_hash(transaction_hash)
            .await
    }

    pub async fn wait_until_transaction_receipt(
        &self,
        transaction_hash: Hash,
    ) -> anyhow::Result<TransactionReceipt> {
        let start_time = std::time::Instant::now();
        let max_retry_time = Duration::from_millis(60_000);

        loop {
            if std::time::Instant::now() > start_time + max_retry_time {
                anyhow::bail!(
                    "failed to find transaction receipt for transaction {}",
                    transaction_hash
                )
            }

            if let Some(transaction_receipt) =
                self.get_transaction_receipt(transaction_hash).await?
            {
                return Ok(transaction_receipt);
            }

            tokio::time::delay_for(Duration::from_millis(1_000)).await;
        }
    }

    pub async fn erc20_balance(&self, token_contract: Address) -> anyhow::Result<Erc20> {
        self.geth_client
            .erc20_balance(self.account(), token_contract)
            .await
    }

    pub async fn dai_balance(&self) -> anyhow::Result<dai::Amount> {
        let balance = self.erc20_balance(self.dai_contract_addr).await?;
        let int = BigUint::from_bytes_le(&balance.quantity.to_bytes());
        Ok(dai::Amount::from_atto(int))
    }

    pub async fn ether_balance(&self) -> anyhow::Result<ether::Amount> {
        self.geth_client.get_balance(self.account()).await
    }

    async fn get_transaction_receipt(
        &self,
        transaction_hash: Hash,
    ) -> anyhow::Result<Option<TransactionReceipt>> {
        self.geth_client
            .get_transaction_receipt(transaction_hash)
            .await
    }

    async fn get_transaction_count(&self) -> anyhow::Result<u32> {
        self.geth_client.get_transaction_count(self.account()).await
    }

    async fn assert_chain(&self, expected: ChainId) -> anyhow::Result<()> {
        let actual = self.geth_client.chain_id().await?;

        if expected != actual {
            anyhow::bail!("Wrong chain_id: expected {:?}, got {:?}", expected, actual);
        }

        Ok(())
    }

    async fn gas_price(&self) -> anyhow::Result<num256::Uint256> {
        self.geth_client.gas_price().await
    }

    async fn gas_limit(&self, request: EstimateGasRequest) -> anyhow::Result<num256::Uint256> {
        self.geth_client.gas_limit(request).await
    }
}

#[derive(Debug, Clone)]
pub struct DeployedContract {
    pub transaction: Transaction,
    pub contract_address: Address,
}

impl From<DeployedContract> for comit::herc20::Deployed {
    fn from(from: DeployedContract) -> Self {
        Self {
            transaction: from.transaction,
            location: from.contract_address,
        }
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::{ethereum::ether, test_harness::ethereum::Blockchain};
    use comit::asset::ethereum::FromWei;
    use comit::asset::{self, Erc20Quantity};

    async fn random_wallet(node_url: Url, dai_contract_address: Address) -> anyhow::Result<Wallet> {
        let seed = Seed::random().unwrap();
        let wallet = Wallet::new(seed, node_url, dai_contract_address, ChainId::regtest()).await?;

        Ok(wallet)
    }

    #[tokio::test]
    async fn ether_balance() {
        let client = testcontainers::clients::Cli::default();

        let mut blockchain = Blockchain::new(&client).unwrap();
        blockchain.init().await.unwrap();

        let wallet = random_wallet(
            blockchain.node_url.clone(),
            blockchain.token_contract().unwrap(),
        )
        .await
        .unwrap();

        let balance = wallet.ether_balance().await.unwrap();

        assert_eq!(balance, ether::Amount::zero())
    }

    #[tokio::test]
    async fn gas_price() {
        let client = testcontainers::clients::Cli::default();

        let mut blockchain = Blockchain::new(&client).unwrap();
        blockchain.init().await.unwrap();

        let wallet = random_wallet(
            blockchain.node_url.clone(),
            blockchain.token_contract().unwrap(),
        )
        .await
        .unwrap();

        let gas_price = wallet.gas_price().await.unwrap();

        println!("Gas price: {}", gas_price)
    }

    #[tokio::test]
    async fn gas_limit() {
        let client = testcontainers::clients::Cli::default();

        let mut blockchain = Blockchain::new(&client).unwrap();
        blockchain.init().await.unwrap();

        let wallet = random_wallet(
            blockchain.node_url.clone(),
            blockchain.token_contract().unwrap(),
        )
        .await
        .unwrap();

        let request = EstimateGasRequest {
            from: Some(Address::random()),
            to: Some(Address::random()),
            gas_price: Some(55_000_000_000u64.into()),
            value: None,
            data: None,
        };

        let gas_limit = wallet.gas_limit(request).await.unwrap();

        println!("Gas limit: {}", gas_limit)
    }

    #[tokio::test]
    async fn transfer_dai() {
        let client = testcontainers::clients::Cli::default();

        let mut blockchain = Blockchain::new(&client).unwrap();
        blockchain.init().await.unwrap();

        let chain_id = blockchain.chain_id();

        let wallet = random_wallet(
            blockchain.node_url.clone(),
            blockchain.token_contract().unwrap(),
        )
        .await
        .unwrap();

        let address = wallet.account();
        let initial_deposit = 5_000_000_000_000_000_000u64;
        blockchain
            .mint_ether(
                address,
                ether::Amount::from_ether_str("2").unwrap(),
                chain_id,
            )
            .await
            .unwrap();

        blockchain
            .mint_erc20_token(
                address,
                Erc20 {
                    quantity: Erc20Quantity::from_wei(initial_deposit),
                    token_contract: wallet.dai_contract_addr,
                },
                chain_id,
            )
            .await
            .unwrap();

        let balance = wallet.dai_balance().await.unwrap();
        assert_eq!(balance, dai::Amount::from_atto(initial_deposit.into()));

        wallet
            .transfer_dai(
                Address::random(),
                dai::Amount::from_dai_trunc(1.0).unwrap(),
                chain_id,
            )
            .await
            .unwrap();

        let balance = wallet.dai_balance().await.unwrap();
        assert_eq!(
            balance,
            dai::Amount::from_atto(4_000_000_000_000_000_000u64.into())
        );
    }

    #[tokio::test]
    async fn can_deploy_htlc() {
        let client = testcontainers::clients::Cli::default();

        let mut blockchain = Blockchain::new(&client).unwrap();
        blockchain.init().await.unwrap();

        let chain_id = blockchain.chain_id();

        let wallet = random_wallet(
            blockchain.node_url.clone(),
            blockchain.token_contract().unwrap(),
        )
        .await
        .unwrap();

        blockchain
            .mint_ether(
                wallet.account(),
                ether::Amount::from_ether_str("2").unwrap(),
                chain_id,
            )
            .await
            .unwrap();

        blockchain
            .mint_erc20_token(
                wallet.account(),
                Erc20 {
                    quantity: Erc20Quantity::from_wei(5_000_000_000_000_000_000u64),
                    token_contract: wallet.dai_contract_addr,
                },
                chain_id,
            )
            .await
            .unwrap();

        let htlc_params = comit::herc20::Params {
            asset: asset::Erc20 {
                token_contract: wallet.dai_contract_addr,
                quantity: Erc20Quantity::from_wei(5_000_000_000u64),
            },
            redeem_identity: Address::random(),
            refund_identity: Address::random(),
            expiry: comit::Timestamp::now(),
            secret_hash: comit::SecretHash::from_vec(b"hello world, you are beautiful!!").unwrap(),
            chain_id,
        };

        wallet
            .deploy_contract(DeployContract {
                data: htlc_params.bytecode(),
                amount: asset::Ether::zero(),
                gas_limit: 160_000,
                chain_id,
            })
            .await
            .unwrap();
    }
}
