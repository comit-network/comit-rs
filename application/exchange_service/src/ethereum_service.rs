use ethereum_htlc::Htlc;
use ethereum_support::*;
use ethereum_wallet;
use gas_price_service;
use gas_price_service::GasPriceService;
use std::ops::DerefMut;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

#[derive(Debug)]
pub enum Error {
    GasPriceUnavailable(gas_price_service::Error),
    NonceLockError,
    Web3(web3::Error),
}

impl From<gas_price_service::Error> for Error {
    fn from(e: gas_price_service::Error) -> Self {
        Error::GasPriceUnavailable(e)
    }
}

impl<'a> From<PoisonError<MutexGuard<'a, U256>>> for Error {
    fn from(_e: PoisonError<MutexGuard<'a, U256>>) -> Self {
        Error::NonceLockError
    }
}

impl<'a> From<web3::Error> for Error {
    fn from(e: web3::Error) -> Self {
        Error::Web3(e)
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

    pub fn deploy_htlc(&self, contract: Htlc, funding: U256) -> Result<H256, Error> {
        let gas_price = self.gas_price_service.get_gas_price()?;

        let tx_id = {
            let mut lock = self.nonce.lock()?;

            let nonce = lock.deref_mut();

            let transaction = ethereum_wallet::UnsignedTransaction::new_contract_deployment(
                contract.compile_to_hex(),
                86578, //TODO: calculate the gas consumption based on 32k + 200*bytes
                gas_price,
                funding,
                *nonce,
            );

            let signed_transaction = self.wallet.sign(&transaction);

            let tx_id = self.web3.send_raw_transaction(signed_transaction.into())?;

            debug!(
                "Contract {:?} was successfully deployed in transaction {:?} with initial funding of {}",
                contract, tx_id, funding
            );

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
    use std::ops::Deref;
    use std::str::FromStr;
    use std::time::SystemTime;

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
    fn given_an_htlc_when_deployment_fails_nonce_is_not_updated() {
        let wallet = ethereum_wallet::fake::StaticFakeWallet::account0();
        let gas_price_service = gas_price_service::StaticGasPriceService::default();
        let ethereum_api = EthereumApiMock::with_result(Err(web3::ErrorKind::Internal.into()));

        let service = EthereumService::new(
            Arc::new(wallet),
            Arc::new(gas_price_service),
            Arc::new(ethereum_api),
            0,
        );

        let result = service.deploy_htlc(
            Htlc::new(
                SystemTime::now(),
                ethereum_support::Address::new(),
                ethereum_support::Address::new(),
                SecretHash::from_str("").unwrap(),
            ),
            U256::from(10),
        );

        let lock = service.nonce.lock().unwrap();
        let nonce = lock.deref();

        assert!(result.is_err());
        assert_eq!(*nonce, U256::from(0))
    }

    #[test]
    fn given_an_htlc_when_deployment_succeeds_nonce_should_be_updated() {
        let wallet = ethereum_wallet::fake::StaticFakeWallet::account0();
        let gas_price_service = gas_price_service::StaticGasPriceService::default();
        let ethereum_api = EthereumApiMock::with_result(Ok(H256::new()));

        let service = EthereumService::new(
            Arc::new(wallet),
            Arc::new(gas_price_service),
            Arc::new(ethereum_api),
            0,
        );

        let result = service.deploy_htlc(
            Htlc::new(
                SystemTime::now(),
                ethereum_support::Address::new(),
                ethereum_support::Address::new(),
                SecretHash::from_str("").unwrap(),
            ),
            U256::from(10),
        );

        let lock = service.nonce.lock().unwrap();
        let nonce = lock.deref();

        assert!(result.is_ok());
        assert_eq!(*nonce, U256::from(1))
    }

}
