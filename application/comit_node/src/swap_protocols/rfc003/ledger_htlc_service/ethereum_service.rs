use ethereum_support::{
    web3::{
        transports::{EventLoopHandle, Http},
        Web3,
    },
    *,
};
use ethereum_wallet::{UnsignedTransaction, Wallet};
use gas_price_service::{self, GasPriceService};
use ledger_query_service::EthereumQuery;
use std::{
    ops::DerefMut,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};
use swap_protocols::{
    ledger::{ethereum::Ethereum, Ledger},
    rfc003::{
        ethereum::{Erc20Htlc, EtherHtlc, Htlc, Seconds},
        ledger_htlc_service::{self, LedgerHtlcService},
        Secret, SecretHash,
    },
};
use swaps::common::TradeId;

impl From<gas_price_service::Error> for ledger_htlc_service::Error {
    fn from(error: gas_price_service::Error) -> Self {
        error!("{:?}", error);
        ledger_htlc_service::Error::Internal
    }
}

impl<'a> From<PoisonError<MutexGuard<'a, U256>>> for ledger_htlc_service::Error {
    fn from(error: PoisonError<MutexGuard<'a, U256>>) -> Self {
        error!("{:?}", error);
        ledger_htlc_service::Error::Internal
    }
}

impl<'a> From<web3::Error> for ledger_htlc_service::Error {
    fn from(error: web3::Error) -> Self {
        error!("{:?}", error);
        ledger_htlc_service::Error::NodeConnection
    }
}

pub trait BlockingEthereumApi: Send + Sync {
    fn send_raw_transaction(&self, rlp: Bytes) -> Result<H256, web3::Error>;
    fn transaction(
        &self,
        transaction_id: TransactionId,
    ) -> Result<Option<Transaction>, web3::Error>;
    fn transaction_receipt(
        &self,
        transaction_id: H256,
    ) -> Result<Option<TransactionReceipt>, web3::Error>;
}

impl BlockingEthereumApi for (EventLoopHandle, Web3<Http>) {
    fn send_raw_transaction(&self, rlp: Bytes) -> Result<H256, web3::Error> {
        self.1.eth().send_raw_transaction(rlp).wait()
    }

    fn transaction(
        &self,
        transaction_id: TransactionId,
    ) -> Result<Option<Transaction>, web3::Error> {
        self.1.eth().transaction(transaction_id).wait()
    }

    fn transaction_receipt(
        &self,
        transaction_id: H256,
    ) -> Result<Option<TransactionReceipt>, web3::Error> {
        self.1.eth().transaction_receipt(transaction_id).wait()
    }
}

#[derive(DebugStub)]
pub struct EthereumService {
    nonce: Mutex<U256>,
    #[debug_stub = "Wallet"]
    wallet: Arc<Wallet>,
    #[debug_stub = "GasPriceService"]
    gas_price_service: Arc<GasPriceService>,
    #[debug_stub = "Web3"]
    web3: Arc<BlockingEthereumApi>,
}

#[derive(Clone, Debug)]
pub struct EtherHtlcFundingParams {
    pub refund_address: Address,
    pub success_address: Address,
    pub time_lock: Seconds,
    pub amount: EtherQuantity,
    pub secret_hash: SecretHash,
}

#[derive(Clone, Debug)]
pub struct EtherHtlcRedeemParams {
    pub contract_address: <Ethereum as Ledger>::HtlcId,
    pub secret: Secret,
}

#[derive(Clone, Debug)]
pub struct Erc20HtlcFundingParams {
    pub refund_address: Address,
    pub success_address: Address,
    pub time_lock: Seconds,
    pub amount: U256,
    pub secret_hash: SecretHash,
    pub token_contract_address: Address,
}

#[derive(Clone, Debug)]
pub struct Erc20HtlcRedeemParams {}

impl LedgerHtlcService<Ethereum, EtherHtlcFundingParams, EtherHtlcRedeemParams, EthereumQuery>
    for EthereumService
{
    fn fund_htlc(
        &self,
        htlc_funding_params: EtherHtlcFundingParams,
    ) -> Result<<Ethereum as Ledger>::TxId, ledger_htlc_service::Error> {
        let contract = EtherHtlc::new(
            htlc_funding_params.time_lock.into(),
            htlc_funding_params.refund_address,
            htlc_funding_params.success_address,
            htlc_funding_params.secret_hash,
        );

        let funding = htlc_funding_params.amount.wei();

        let tx_id = self.sign_and_send(|nonce, gas_price| {
            UnsignedTransaction::new_contract_deployment(
                contract.compile_to_hex(),
                gas_price,
                funding,
                nonce,
                None,
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
        _trade_id: TradeId,
        htlc_redeem_params: EtherHtlcRedeemParams,
    ) -> Result<<Ethereum as Ledger>::TxId, ledger_htlc_service::Error> {
        let contract_address = htlc_redeem_params.contract_address;
        let secret = htlc_redeem_params.secret;

        let tx_id = self.sign_and_send(|nonce, gas_price| {
            UnsignedTransaction::new_contract_invocation(
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

    fn create_query_to_watch_redeeming(
        &self,
        htlc_funding_tx_id: <Ethereum as Ledger>::TxId,
    ) -> Result<EthereumQuery, ledger_htlc_service::Error> {
        match self.get_contract_address(htlc_funding_tx_id) {
            Ok(Some(eth_htlc_address)) => Ok(EthereumQuery {
                from_address: None,
                to_address: Some(eth_htlc_address),
                is_contract_creation: None,
                transaction_data: None,
            }),
            _ => Err(ledger_htlc_service::Error::Internal),
        }
    }

    fn create_query_to_watch_funding(&self, htlc_params: EtherHtlcFundingParams) -> EthereumQuery {
        let data = EtherHtlc::new(
            htlc_params.time_lock.into(),
            htlc_params.refund_address,
            htlc_params.success_address,
            htlc_params.secret_hash,
        ).compile_to_hex()
        .into();

        EthereumQuery {
            from_address: None,
            to_address: None,
            is_contract_creation: Some(true),
            transaction_data: Some(data),
        }
    }

    fn check_and_extract_secret(
        &self,
        create_htlc_tx_id: <Ethereum as Ledger>::TxId,
        redeem_htlc_tx_id: <Ethereum as Ledger>::TxId,
    ) -> Result<Secret, ledger_htlc_service::Error> {
        let htlc_address = match self.get_contract_address(create_htlc_tx_id) {
            Ok(Some(address)) => address,
            _ => return Err(ledger_htlc_service::Error::Internal),
        };

        let redeem_tx = self
            .web3
            .transaction(TransactionId::Hash(redeem_htlc_tx_id))?;
        match redeem_tx {
            None => {
                error!(
                    "Could not get details of transaction {:#?}",
                    redeem_htlc_tx_id
                );
                Err(ledger_htlc_service::Error::TransactionNotFound)
            }
            Some(tx) => {
                match tx.to {
                    Some(address) => {
                        if address == htlc_address {
                            // TODO: Check this is the transaction that redeems the contract. See #316
                            let data = tx.input.0;
                            debug!("Transaction data: {:?}", data);
                            match Secret::from_vec(&data) {
                                Err(_) => {
                                    error!("Could not get secret out of transaction data");
                                    Err(ledger_htlc_service::Error::InvalidRedeemTransaction)
                                }
                                Ok(secret) => Ok(secret),
                            }
                        } else {
                            error!(
                                "Expected 'to' is {:?} not {:?} for transaction {:?}",
                                htlc_address, address, tx
                            );
                            Err(ledger_htlc_service::Error::InvalidRedeemTransaction)
                        }
                    }
                    _ => {
                        error!("'to' is expected on redeem transaction {:?}", tx);
                        Err(ledger_htlc_service::Error::InvalidRedeemTransaction)
                    }
                }
            }
        }
    }
}

impl LedgerHtlcService<Ethereum, Erc20HtlcFundingParams, Erc20HtlcRedeemParams, EthereumQuery>
    for EthereumService
{
    fn fund_htlc(
        &self,
        htlc_funding_params: Erc20HtlcFundingParams,
    ) -> Result<<Ethereum as Ledger>::TxId, ledger_htlc_service::Error> {
        let gas_price = self.gas_price_service.get_gas_price()?;

        let tx_id = {
            let mut lock = self.nonce.lock()?;

            let nonce = lock.deref_mut();

            let htlc_address = self
                .wallet
                .calculate_contract_address(*nonce + U256::from(1));

            let approval_transaction = UnsignedTransaction::new_erc20_approval(
                htlc_funding_params.token_contract_address,
                htlc_address,
                htlc_funding_params.amount,
                gas_price,
                *nonce,
            );

            let signed_approval_transaction = self.wallet.sign(&approval_transaction);

            let _tx_id = self
                .web3
                .send_raw_transaction(signed_approval_transaction.into())?;

            EthereumService::increment_nonce(nonce);

            let htlc = Erc20Htlc::new(
                htlc_funding_params.time_lock.into(),
                htlc_funding_params.refund_address,
                htlc_funding_params.success_address,
                htlc_funding_params.secret_hash,
                htlc_address,
                htlc_funding_params.token_contract_address,
                htlc_funding_params.amount,
            );

            let htlc_code = htlc.compile_to_hex();

            let deployment_transaction = UnsignedTransaction::new_contract_deployment(
                htlc_code,
                gas_price,
                0,
                *nonce,
                Some(100_000),
            );

            let signed_deployment_transaction = self.wallet.sign(&deployment_transaction);

            let tx_id = self
                .web3
                .send_raw_transaction(signed_deployment_transaction.into())?;

            EthereumService::increment_nonce(nonce);

            tx_id
        };

        Ok(tx_id)
    }

    fn redeem_htlc(
        &self,
        _trade_id: TradeId,
        _htlc_redeem_params: Erc20HtlcRedeemParams,
    ) -> Result<<Ethereum as Ledger>::TxId, ledger_htlc_service::Error> {
        unimplemented!()
    }

    fn create_query_to_watch_redeeming(
        &self,
        _htlc_funding_tx_id: <Ethereum as Ledger>::TxId,
    ) -> Result<EthereumQuery, ledger_htlc_service::Error> {
        unimplemented!()
    }

    fn create_query_to_watch_funding(&self, _htlc_params: Erc20HtlcFundingParams) -> EthereumQuery {
        unimplemented!()
    }

    fn check_and_extract_secret(
        &self,
        _create_htlc_tx_id: <Ethereum as Ledger>::TxId,
        _redeem_htlc_tx_id: <Ethereum as Ledger>::TxId,
    ) -> Result<Secret, ledger_htlc_service::Error> {
        unimplemented!()
    }
}

impl EthereumService {
    pub fn new<N: Into<U256>>(
        wallet: Arc<Wallet>,
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

    fn sign_and_send<T: Fn(U256, U256) -> UnsignedTransaction>(
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

    pub fn get_contract_address(&self, deploy_tx_id: H256) -> Result<Option<Address>, web3::Error> {
        self.web3
            .transaction_receipt(deploy_tx_id)
            .map(|opt| opt.and_then(|receipt| receipt.contract_address))
    }

    pub fn get_transaction(&self, tx_id: H256) -> Result<Option<Transaction>, web3::Error> {
        self.web3.transaction(TransactionId::Hash(tx_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_wallet::{fake::StaticFakeWallet, Wallet};
    use hex;
    use secp256k1_support::KeyPair;
    use spectral::prelude::*;
    use std::ops::Deref;
    use swap_protocols::rfc003::ethereum::Seconds;

    struct MockEthereumApi {
        send_raw_transaction_results: Mutex<Vec<Result<H256, web3::Error>>>,
        sent_bytes: Mutex<Vec<Bytes>>,
    }

    impl MockEthereumApi {
        fn with_result(results: Vec<Result<H256, web3::Error>>) -> Self {
            MockEthereumApi {
                send_raw_transaction_results: Mutex::new(results),
                sent_bytes: Mutex::new(Vec::new()),
            }
        }
    }

    impl BlockingEthereumApi for MockEthereumApi {
        fn send_raw_transaction(&self, rlp: Bytes) -> Result<H256, web3::Error> {
            let mut results = self.send_raw_transaction_results.lock().unwrap();
            let mut sent_bytes = self.sent_bytes.lock().unwrap();

            sent_bytes.push(rlp);

            results.remove(0)
        }

        fn transaction(
            &self,
            _transaction_id: TransactionId,
        ) -> Result<Option<Transaction>, web3::Error> {
            unimplemented!()
        }

        fn transaction_receipt(
            &self,
            _transaction_id: H256,
        ) -> Result<Option<TransactionReceipt>, web3::Error> {
            unimplemented!()
        }
    }

    #[test]
    fn given_a_transaction_when_deployment_fails_nonce_is_not_updated() {
        let wallet = StaticFakeWallet::account0();
        let gas_price_service = gas_price_service::StaticGasPriceService::default();
        let ethereum_api =
            MockEthereumApi::with_result(vec![Err(web3::ErrorKind::Internal.into())]);

        let service = EthereumService::new(
            Arc::new(wallet),
            Arc::new(gas_price_service),
            Arc::new(ethereum_api),
            0,
        );

        let result = service.sign_and_send(|nonce, gas_price| {
            UnsignedTransaction::new_contract_deployment(
                EtherHtlc::new(
                    Seconds(100),
                    Address::new(),
                    Address::new(),
                    "".parse().unwrap(),
                ).compile_to_hex(),
                gas_price,
                U256::from(10),
                nonce,
                None,
            )
        });

        let lock = service.nonce.lock().unwrap();
        let nonce = lock.deref();

        assert!(result.is_err());
        assert_eq!(*nonce, U256::from(0))
    }

    #[test]
    fn given_a_transaction_when_deployment_succeeds_nonce_should_be_updated() {
        let wallet = StaticFakeWallet::account0();
        let gas_price_service = gas_price_service::StaticGasPriceService::default();
        let ethereum_api = MockEthereumApi::with_result(vec![Ok(H256::new())]);

        let service = EthereumService::new(
            Arc::new(wallet),
            Arc::new(gas_price_service),
            Arc::new(ethereum_api),
            0,
        );

        let result = service.sign_and_send(|nonce, gas_price| {
            UnsignedTransaction::new_contract_deployment(
                EtherHtlc::new(
                    Seconds(100),
                    Address::new(),
                    Address::new(),
                    "".parse().unwrap(),
                ).compile_to_hex(),
                gas_price,
                U256::from(10),
                nonce,
                None,
            )
        });

        let lock = service.nonce.lock().unwrap();
        let nonce = lock.deref();

        assert!(result.is_ok());
        assert_eq!(*nonce, U256::from(1))
    }

    #[test]
    fn given_erc20htlcparams_when_deploy_htlc_is_called_sends_two_transactions() {
        // First, initialize the wallet with a known secret key. This way, we know the address of this account. It is: 0x94e4782ae2db9bce7ac1920869f420026ca58f33
        let keypair = KeyPair::from_secret_key_slice(
            &hex::decode("29b7de7fed2f25726c247b70fc51e73ab03398d230da42e8a550e405e744ed7a")
                .unwrap(),
        ).unwrap();
        let wallet = Arc::new(StaticFakeWallet::from_key_pair(keypair));

        let gas_price_service = gas_price_service::StaticGasPriceService::new(1000);
        let tx_1 = H256::from("0000000000000000000000000000000000000000000000000000000000000001");
        let tx_2 = H256::from("0000000000000000000000000000000000000000000000000000000000000002");
        let ethereum_api = Arc::new(MockEthereumApi::with_result(vec![Ok(tx_1), Ok(tx_2)]));
        let service = EthereumService::new(
            wallet.clone(),
            Arc::new(gas_price_service),
            ethereum_api.clone(),
            0,
        );

        let params = Erc20HtlcFundingParams {
            refund_address: Address::from("0000000000000000000000000000000000000001"),
            success_address: Address::from("0000000000000000000000000000000000000002"),
            time_lock: Seconds(100),
            amount: U256::from(10),
            secret_hash: "".parse().unwrap(),
            token_contract_address: Address::from("0000000000000000000000000000000000000003"),
        };

        // Act
        let result = service.fund_htlc(params.clone());

        // Assert
        let sent_bytes = ethereum_api.sent_bytes.lock().unwrap();

        assert_that(&result).is_ok().is_equal_to(&tx_2);

        // The first transaction needs to approve the to-be-deployed contract which already includes the contract address.
        // The contract will be deployed next. Therefore the contract address will be derived from the account address + (current_nonce + 1).
        let erc20_approval = UnsignedTransaction::new_erc20_approval(
            Address::from("0000000000000000000000000000000000000003"),
            Address::from("97a561cef28e387e726378bb41d89b13e5a940ba"),
            10,
            1000,
            0,
        );

        assert_that(&*sent_bytes).contains(&Bytes::from(wallet.sign(&erc20_approval)));

        let htlc_deployment = UnsignedTransaction::new_contract_deployment(
            Erc20Htlc::new(
                params.time_lock.into(),
                params.refund_address,
                params.success_address,
                params.secret_hash,
                Address::from("97a561cef28e387e726378bb41d89b13e5a940ba"),
                params.token_contract_address,
                params.amount,
            ).compile_to_hex(),
            1000,
            0,
            1,
            Some(100_000),
        );

        assert_that(&*sent_bytes).contains(&Bytes::from(wallet.sign(&htlc_deployment)));

        let nonce = service.nonce.lock().unwrap();
        assert_that(&*nonce).is_equal_to(&U256::from(2));
    }
}
