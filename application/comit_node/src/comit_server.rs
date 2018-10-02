use bitcoin_support::{BitcoinQuantity, Network};
use comit_wallet::KeyStore;
use ethereum_support::EthereumQuantity;
use event_store::EventStore;
use futures::{Future, Stream};
use ledger_query_service::{BitcoinQuery, LedgerQueryServiceApiClient};
use std::{io, net::SocketAddr, sync::Arc};
use swap_protocols::{
    json_config,
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003::{self, ledger_htlc_service::EthereumService},
    wire_types::SwapResponse,
    SwapRequestHandler,
};
use swaps::common::TradeId;
use tokio::{self, net::TcpListener};
use transport_protocol::{connection::Connection, json};

pub struct ComitServer<
    E: EventStore<TradeId> + Send + Sync,
    BLQS: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>,
> {
    event_store: Arc<E>,
    my_keystore: Arc<KeyStore>,
    ethereum_service: Arc<EthereumService>,
    bitcoin_network: Network,
    ledger_query_service: Arc<BLQS>,
}

impl<E, BLQS> ComitServer<E, BLQS>
where
    E: EventStore<TradeId> + Send + Sync,
    BLQS: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>, // + LedgerQueryServiceApiClient<Ethereum, EthereumQuery>
{
    pub fn new(
        event_store: Arc<E>,
        my_keystore: Arc<KeyStore>,
        ethereum_service: Arc<EthereumService>,
        bitcoin_network: Network,
        ledger_query_service: Arc<BLQS>,
    ) -> Self {
        Self {
            event_store,
            my_keystore,
            ethereum_service,
            bitcoin_network,
            ledger_query_service,
        }
    }

    pub fn listen(self, addr: SocketAddr) -> impl Future<Item = (), Error = io::Error> {
        info!("ComitServer listening at {:?}", addr);
        let socket = TcpListener::bind(&addr).unwrap();

        socket.incoming().for_each(move |connection| {
            let peer_addr = connection.peer_addr();
            let codec = json::JsonFrameCodec::default();

            let swap_handler = MySwapHandler::default();

            let config = json_config(
                swap_handler,
                self.my_keystore.clone(),
                self.event_store.clone(),
                self.ledger_query_service.clone(),
                self.ethereum_service.clone(),
                self.bitcoin_network,
            );
            let connection = Connection::new(config, codec, connection);
            let (close_future, _client) = connection.start::<json::JsonFrameHandler>();
            tokio::spawn(close_future.map_err(move |e| {
                error!(
                    "Unexpected error in connection with {:?}: {:?}",
                    peer_addr, e
                );
            }));
            Ok(())
        })
    }
}

#[derive(Default)]
struct MySwapHandler {}

impl SwapRequestHandler<rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>>
    for MySwapHandler
{
    fn handle(
        &mut self,
        _request: rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
    ) -> SwapResponse {
        {
            // TODO: Decide whether to swap
            SwapResponse::Accept
        }
    }
}

impl SwapRequestHandler<rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>>
    for MySwapHandler
{
    fn handle(
        &mut self,
        _request: rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>,
    ) -> SwapResponse {
        {
            SwapResponse::Decline
        }
    }
}
