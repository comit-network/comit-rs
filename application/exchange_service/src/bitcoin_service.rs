use bitcoin_htlc::bitcoin_htlc::Htlc;
use bitcoin_rpc_client::{self, RpcError, TransactionId};
use bitcoin_support::{self, Address};
use reqwest;
use std::sync::Arc;

#[derive(Debug)]
pub enum Error {
    BitcoinRpc(bitcoin_rpc_client::RpcError),
    BitcoinNode(reqwest::Error),
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::BitcoinNode(error)
    }
}

impl From<bitcoin_rpc_client::RpcError> for Error {
    fn from(error: bitcoin_rpc_client::RpcError) -> Self {
        Error::BitcoinRpc(error)
    }
}

pub trait LocalBitcoinApi: Send + Sync {
    fn deploy_htlc(
        &self,
        address: &Address,
        amount: f64,
    ) -> Result<Result<TransactionId, RpcError>, reqwest::Error>;
}

impl<T> LocalBitcoinApi for T
where
    T: bitcoin_rpc_client::BitcoinRpcApi,
{
    fn deploy_htlc(
        &self,
        address: &Address,
        amount: f64,
    ) -> Result<Result<TransactionId, RpcError>, reqwest::Error> {
        self.send_to_address(&address.clone().into(), amount)
    }
}

pub struct BitcoinService {
    client: Arc<LocalBitcoinApi>,
    network: bitcoin_support::Network,
}

impl BitcoinService {
    pub fn new(client: Arc<LocalBitcoinApi>, network: bitcoin_support::Network) -> Self {
        BitcoinService { client, network }
    }

    pub fn deploy_htlc(
        &self,
        contract: Htlc,
        amount: bitcoin_support::BitcoinQuantity,
    ) -> Result<TransactionId, Error> {
        let htlc_address = contract.compute_address(self.network);

        let tx_id = self
            .client
            .deploy_htlc(&htlc_address.clone().into(), amount.bitcoin())??;

        Ok(tx_id)
    }
}
