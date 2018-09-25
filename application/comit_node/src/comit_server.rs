use bitcoin_payment_future::LedgerServices;
use bitcoin_rpc_client::BitcoinRpcApi;
use bitcoin_support::BitcoinQuantity;
use comit_wallet::KeyStore;
use ethereum_support::EthereumQuantity;
use event_store::InMemoryEventStore;
use futures::{Future, Stream};
use futures_ext::FutureFactory;
use ledger_query_service::{
    BitcoinQuery, DefaultLedgerQueryServiceApiClient, LedgerQueryServiceApiClient,
};
use std::{io, net::SocketAddr, sync::Arc, time::Duration};
use swap_protocols::{
    json_config,
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003::{self, ledger_htlc_service::EthereumService},
    wire_types::SwapResponse,
    SwapRequestHandler,
};
use swaps::common::TradeId;
use tokio::{self, net::TcpListener, runtime::Runtime};
use transport_protocol::{connection::Connection, json};

pub struct ComitServer {
    event_store: Arc<InMemoryEventStore<TradeId>>,
    my_keystore: Arc<KeyStore>,
}

impl ComitServer {
    pub fn new(event_store: Arc<InMemoryEventStore<TradeId>>, my_keystore: Arc<KeyStore>) -> Self {
        Self {
            event_store,
            my_keystore,
        }
    }

    pub fn listen(
        self,
        addr: SocketAddr,
        bitcoin_node: Arc<BitcoinRpcApi>,
        ethereum_service: Arc<EthereumService>,
    ) -> impl Future<Item = (), Error = io::Error> {
        info!("ComitServer listening at {:?}", addr);
        let socket = TcpListener::bind(&addr).unwrap();

        socket.incoming().for_each(move |connection| {
            let peer_addr = connection.peer_addr();
            let codec = json::JsonFrameCodec::default();

            // TODO: Apparently Runtime::run() is better
            let runtime = Runtime::new().unwrap();

            //TODO: obviously need to get url from parameters
            let ledger_query_service = Arc::new(DefaultLedgerQueryServiceApiClient::new(
                "http://bitcoin_ledger_service.com/".parse().unwrap(),
            ));

            // TODO: Duration to come from somewhere else
            let ledger_services =
                LedgerServices::new(ledger_query_service.clone(), Duration::from_millis(100));

            let future_factory = FutureFactory::new(ledger_services);

            let swap_handler = MySwapHandler::new(
                self.my_keystore.clone(),
                self.event_store.clone(),
                runtime,
                future_factory,
                ledger_query_service.clone(),
                bitcoin_node.clone(),
                ethereum_service.clone(),
            );

            let config = json_config(
                swap_handler,
                self.my_keystore.clone(),
                self.event_store.clone(),
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

struct MySwapHandler<C> {
    my_keystore: Arc<KeyStore>,
    event_store: Arc<InMemoryEventStore<TradeId>>,
    runtime: Runtime,
    future_factory: FutureFactory<LedgerServices>,
    // TODO: Do we really need that as it is inside the factory?
    ledger_query_service_api_client: Arc<C>,
    bitcoin_node: Arc<BitcoinRpcApi>,
    ethereum_service: Arc<EthereumService>,
}

impl<C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>> MySwapHandler<C> {
    pub fn new(
        my_keystore: Arc<KeyStore>,
        event_store: Arc<InMemoryEventStore<TradeId>>,
        runtime: Runtime,
        future_factory: FutureFactory<LedgerServices>,
        ledger_query_service_api_client: Arc<C>,
        bitcoin_node: Arc<BitcoinRpcApi>,
        ethereum_service: Arc<EthereumService>,
    ) -> Self {
        MySwapHandler {
            my_keystore,
            event_store,
            runtime,
            future_factory,
            ledger_query_service_api_client,
            bitcoin_node,
            ethereum_service,
        }
    }
}

impl<C: 'static + LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>>
    SwapRequestHandler<rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>>
    for MySwapHandler<C>
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
impl<C: 'static + LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>>
    SwapRequestHandler<rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>>
    for MySwapHandler<C>
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
