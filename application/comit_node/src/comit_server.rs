use bitcoin_payment_future::LedgerServices;
use bitcoin_support::BitcoinQuantity;
use comit_wallet::KeyStore;
use ethereum_support::EthereumQuantity;
use event_store::InMemoryEventStore;
use futures::{Future, Stream};
use futures_ext::FutureFactory;
use ledger_query_service::DefaultLedgerQueryServiceApiClient;
use std::{io, net::SocketAddr, sync::Arc, time::Duration};
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

pub struct ComitServer {
    event_store: Arc<InMemoryEventStore<TradeId>>,
    my_keystore: Arc<KeyStore>,
    ethereum_service: Arc<EthereumService>,
}

impl ComitServer {
    pub fn new(
        event_store: Arc<InMemoryEventStore<TradeId>>,
        my_keystore: Arc<KeyStore>,
        ethereum_service: Arc<EthereumService>,
    ) -> Self {
        Self {
            event_store,
            my_keystore,
            ethereum_service,
        }
    }

    pub fn listen(self, addr: SocketAddr) -> impl Future<Item = (), Error = io::Error> {
        info!("ComitServer listening at {:?}", addr);
        let socket = TcpListener::bind(&addr).unwrap();

        socket.incoming().for_each(move |connection| {
            let peer_addr = connection.peer_addr();
            let codec = json::JsonFrameCodec::default();

            //TODO: Pass ledger query service in Comitserver
            let ledger_query_service = Arc::new(DefaultLedgerQueryServiceApiClient::new(
                "http://bitcoin_ledger_service.com/".parse().unwrap(),
            ));

            // TODO: Duration to come from somewhere else
            let ledger_services =
                LedgerServices::new(ledger_query_service.clone(), Duration::from_millis(100));

            //TODO: not sure this Arc is needed but getting an "outer capture error" in json_config
            let future_factory = Arc::new(FutureFactory::new(ledger_services));

            let swap_handler = MySwapHandler::default();

            let config = json_config(
                swap_handler,
                self.my_keystore.clone(),
                self.event_store.clone(),
                future_factory.clone(),
                ledger_query_service.clone(),
                self.ethereum_service.clone(),
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

//TODO: Should be Ethereum, EthereumQuery
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
