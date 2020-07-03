use crate::{geth, Seed};
use comit::{
    actions::ethereum::{CallContract, DeployContract},
    asset::Erc20,
    ethereum::{Address, ChainId, Hash, TransactionReceipt},
};
use reqwest::Url;
use std::str::FromStr;

// TODO: Add network; assert network on all calls to geth
#[derive(Debug, Clone)]
pub struct Wallet {
    private_key: clarity::PrivateKey,
    geth_client: geth::Client,
    dai_contract_adr: Address,
}

impl Wallet {
    pub fn new(seed: Seed, url: Url) -> anyhow::Result<Self> {
        let private_key = clarity::PrivateKey::from_slice(&seed.secret_key_bytes())
            .map_err(|_| anyhow::anyhow!("Failed to derive private key from slice"))?;

        let geth_client = geth::Client::new(url);

        // TODO: Properly deal with address according to chain-id (currently set to mainnet address)
        let dai_contract_adr = Address::from_str("6b175474e89094c44da98b954eedeac495271d0f")
            .expect("dai contract address");

        Ok(Self {
            private_key,
            geth_client,
            dai_contract_adr,
        })
    }

    #[cfg(test)]
    pub fn new_from_private_key(private_key: clarity::PrivateKey, url: Url) -> Self {
        let geth_client = geth::Client::new(url);
        let dai_contract_adr = Address::random();

        Self {
            private_key,
            geth_client,
            dai_contract_adr,
        }
    }

    pub fn account(&self) -> Address {
        let pk = self.private_key.to_public_key().expect("cannot fail");

        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(pk.as_bytes());

        Address::from(bytes)
    }

    pub async fn deploy_contract(
        &self,
        DeployContract {
            data,
            gas_limit,
            chain_id,
            ..
        }: DeployContract,
    ) -> anyhow::Result<Hash> {
        let nonce = self.get_transaction_count().await?;

        let transaction = clarity::Transaction {
            nonce: nonce.into(),
            gas_price: 0u32.into(),
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

        // TODO: Return TransactionReceipt instead
        std::thread::sleep(std::time::Duration::from_millis(2000));

        Ok(hash)
    }

    pub async fn send_transaction(
        &self,
        to: Address,
        value: u64,
        gas_limit: u64,
        data: Option<Vec<u8>>,
        chain_id: ChainId,
    ) -> anyhow::Result<Hash> {
        let nonce = self.get_transaction_count().await?;

        let transaction = clarity::Transaction {
            nonce: nonce.into(),
            gas_price: 0u32.into(),
            gas_limit: gas_limit.into(),
            to: clarity::Address::from_slice(to.as_bytes()).map_err(|_| {
                anyhow::anyhow!("Failed to deserialize slice into clarity::Address")
            })?,
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
        let nonce = self.get_transaction_count().await?;

        let transaction = clarity::Transaction {
            nonce: nonce.into(),
            gas_price: 0u32.into(),
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
    ) -> anyhow::Result<comit::ethereum::Transaction> {
        self.geth_client
            .get_transaction_by_hash(transaction_hash)
            .await
    }

    pub async fn get_transaction_receipt(
        &self,
        transaction_hash: Hash,
    ) -> anyhow::Result<TransactionReceipt> {
        self.geth_client
            .get_transaction_receipt(transaction_hash)
            .await
    }

    // QUESTION: Do we need to handle decimal places?
    pub async fn erc20_balance(&self, token_contract: Address) -> anyhow::Result<Erc20> {
        self.geth_client
            .erc20_balance(self.account(), token_contract)
            .await
    }

    pub async fn dai_balance(&self) -> anyhow::Result<Erc20> {
        self.erc20_balance(self.dai_contract_adr).await
    }

    pub async fn get_transaction_count(&self) -> anyhow::Result<u32> {
        self.geth_client.get_transaction_count(self.account()).await
    }
}
