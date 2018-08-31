use common_types::{
    ledger::{ethereum::Ethereum, Ledger},
    secret::{Secret, SecretHash},
};
use ethereum_htlc::Htlc;
use ethereum_support::*;
use ethereum_wallet;
use gas_price_service::{self, GasPriceService};
use ledger_htlc_service::{self, LedgerHtlcService};
use secp256k1_support::KeyPair;
use std::{
    ops::DerefMut,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};
use swaps::common::TradeId;

impl From<gas_price_service::Error> for ledger_htlc_service::Error {
    fn from(_e: gas_price_service::Error) -> Self {
        ledger_htlc_service::Error::Internal
    }
}

impl<'a> From<PoisonError<MutexGuard<'a, U256>>> for ledger_htlc_service::Error {
    fn from(_e: PoisonError<MutexGuard<'a, U256>>) -> Self {
        ledger_htlc_service::Error::Internal
    }
}

impl<'a> From<web3::Error> for ledger_htlc_service::Error {
    fn from(_e: web3::Error) -> Self {
        ledger_htlc_service::Error::NodeConnection
    }
}

pub trait BlockingEthereumApi: Send + Sync {
    fn send_raw_transaction(&self, rlp: Bytes) -> Result<H256, web3::Error>;
}

impl BlockingEthereumApi for Web3Client {
    fn send_raw_transaction(&self, rlp: Bytes) -> Result<H256, web3::Error> {
        let result = self.eth().send_raw_transaction(rlp).wait();

        result
    }
}

pub struct EthereumService {
    nonce: Mutex<U256>,
    wallet: Arc<ethereum_wallet::Wallet>,
    gas_price_service: Arc<GasPriceService>,
    web3: Arc<BlockingEthereumApi>,
}

impl LedgerHtlcService<Ethereum> for EthereumService {
    fn deploy_htlc(
        &self,
        refund_address: <Ethereum as Ledger>::Address,
        success_address: <Ethereum as Ledger>::Address,
        time_lock: <Ethereum as Ledger>::LockDuration,
        amount: <Ethereum as Ledger>::Quantity,
        secret: SecretHash,
    ) -> Result<<Ethereum as Ledger>::TxId, ledger_htlc_service::Error> {
        let contract = Htlc::new(time_lock.into(), refund_address, success_address, secret);

        let funding = amount.wei();

        let tx_id = self.sign_and_send(|nonce, gas_price| {
            ethereum_wallet::UnsignedTransaction::new_contract_deployment(
                contract.compile_to_hex(),
                865780, //TODO: calculate the gas consumption based on 32k + 200*bytes
                gas_price,
                funding,
                nonce,
            )
        })?;

        info!(
            "Contract {:?} was successfully deployed in transaction {:?} with initial funding of {}",
            contract, tx_id, funding
        );

        Ok(tx_id)
    }

    fn redeem_htlc(
        &self,
        secret: Secret,
        _trade_id: TradeId,
        _exchange_success_address: <Ethereum as Ledger>::Address,
        _exchange_success_keypair: KeyPair,
        _client_refund_address: <Ethereum as Ledger>::Address,
        contract_address: <Ethereum as Ledger>::HtlcId,
        _sell_amount: <Ethereum as Ledger>::Quantity,
        _lock_time: <Ethereum as Ledger>::LockDuration,
    ) -> Result<<Ethereum as Ledger>::TxId, ledger_htlc_service::Error> {
        let tx_id = self.sign_and_send(|nonce, gas_price| {
            ethereum_wallet::UnsignedTransaction::new_contract_invocation(
                secret.raw_secret().to_vec(),
                contract_address,
                10000,
                gas_price,
                0,
                nonce,
            )
        })?;

        info!(
            "Contract {:?} was successfully redeemed in transaction {:?}",
            contract_address, tx_id
        );

        Ok(tx_id)
    }
}

impl EthereumService {
    pub fn new<N: Into<U256>>(
        wallet: Arc<ethereum_wallet::Wallet>,
        gas_price_service: Arc<GasPriceService>,
        web3: Arc<BlockingEthereumApi>,
        current_nonce: N,
    ) -> Self {
        EthereumService {
            wallet,
            nonce: Mutex::new(current_nonce.into()),
            gas_price_service,
            web3,
        }
    }

    fn sign_and_send<T: Fn(U256, U256) -> ethereum_wallet::UnsignedTransaction>(
        &self,
        transaction_fn: T,
    ) -> Result<H256, ledger_htlc_service::Error> {
        let gas_price = self.gas_price_service.get_gas_price()?;

        let tx_id = {
            let mut lock = self.nonce.lock()?;

            let nonce = lock.deref_mut();

            let transaction = transaction_fn(*nonce, gas_price);

            let signed_transaction = self.wallet.sign(&transaction);

            let tx_id = self.web3.send_raw_transaction(signed_transaction.into())?;

            // If we get this far, everything worked.
            // Update the nonce and release the lock.
            EthereumService::increment_nonce(nonce);

            tx_id
        };

        Ok(tx_id)
    }

    fn increment_nonce(nonce: &mut U256) {
        let next_nonce = *nonce + U256::from(1);
        debug!("Nonce was incremented from {} to {}", nonce, next_nonce);
        *nonce = next_nonce;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common_types::secret::SecretHash;
    use ethereum_support;
    use std::{ops::Deref, str::FromStr, time::Duration};

    struct EthereumApiMock {
        result: Result<H256, web3::Error>,
    }

    impl EthereumApiMock {
        fn with_result(result: Result<H256, web3::Error>) -> Self {
            EthereumApiMock { result }
        }
    }

    impl BlockingEthereumApi for EthereumApiMock {
        fn send_raw_transaction(&self, _rlp: Bytes) -> Result<H256, web3::Error> {
            self.result.clone()
        }
    }

    // This is a test where we know that the instance is always accessed only from one thread.
    // Thus it is safe although the mock takes a mutable reference.
    unsafe impl Send for EthereumApiMock {}

    unsafe impl Sync for EthereumApiMock {}

    #[test]
    fn given_a_transaction_when_deployment_fails_nonce_is_not_updated() {
        let wallet = ethereum_wallet::fake::StaticFakeWallet::account0();
        let gas_price_service = gas_price_service::StaticGasPriceService::default();
        let ethereum_api = EthereumApiMock::with_result(Err(web3::ErrorKind::Internal.into()));

        let service = EthereumService::new(
            Arc::new(wallet),
            Arc::new(gas_price_service),
            Arc::new(ethereum_api),
            0,
        );

        let result = service.sign_and_send(|nonce, gas_price| {
            ethereum_wallet::UnsignedTransaction::new_contract_deployment(
                Htlc::new(
                    Duration::from_secs(100),
                    ethereum_support::Address::new(),
                    ethereum_support::Address::new(),
                    SecretHash::from_str("").unwrap(),
                ).compile_to_hex(),
                86578,
                gas_price,
                U256::from(10),
                nonce,
            )
        });

        let lock = service.nonce.lock().unwrap();
        let nonce = lock.deref();

        assert!(result.is_err());
        assert_eq!(*nonce, U256::from(0))
    }

    #[test]
    fn given_a_transaction_when_deployment_succeeds_nonce_should_be_updated() {
        let wallet = ethereum_wallet::fake::StaticFakeWallet::account0();
        let gas_price_service = gas_price_service::StaticGasPriceService::default();
        let ethereum_api = EthereumApiMock::with_result(Ok(H256::new()));

        let service = EthereumService::new(
            Arc::new(wallet),
            Arc::new(gas_price_service),
            Arc::new(ethereum_api),
            0,
        );

        let result = service.sign_and_send(|nonce, gas_price| {
            ethereum_wallet::UnsignedTransaction::new_contract_deployment(
                Htlc::new(
                    Duration::from_secs(100),
                    ethereum_support::Address::new(),
                    ethereum_support::Address::new(),
                    SecretHash::from_str("").unwrap(),
                ).compile_to_hex(),
                86578,
                gas_price,
                U256::from(10),
                nonce,
            )
        });

        let lock = service.nonce.lock().unwrap();
        let nonce = lock.deref();

        assert!(result.is_ok());
        assert_eq!(*nonce, U256::from(1))
    }
}
