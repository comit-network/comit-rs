use bitcoin_payment_future::LedgerServices;
use bitcoin_rpc_client::BitcoinRpcApi;
use bitcoin_support::{BitcoinQuantity, ToP2wpkhAddress};
use common_types::seconds::Seconds;
use ethereum_support::{self, EthereumQuantity};
use event_store::{EventStore, InMemoryEventStore};
use futures::{Future, Stream};
use futures_ext::FutureFactory;
use ganp::{
    self,
    ledger::{
        bitcoin::{Bitcoin, HtlcId},
        ethereum::Ethereum,
    },
    rfc003, swap, SwapRequestHandler,
};
use ledger_query_service::{BitcoinQuery, LedgerQueryServiceApiClient};
use secp256k1_support::KeyPair;
use std::{io, net::SocketAddr, sync::Arc};
use swaps::{
    alice_events::ContractDeployed as AliceContractDeployed,
    bob_events::{
        ContractDeployed as BobContractDeployed, ContractRedeemed as BobContractRedeemed,
        OrderTaken, OrderTaken as BobOrderTaken, TradeFunded as BobTradeFunded,
    },
    common::TradeId,
    errors::Error,
};
use tokio::{self, net::TcpListener, runtime::Runtime};
use transport_protocol::{connection::Connection, json};

pub struct ComitServer {
    event_store: Arc<InMemoryEventStore<TradeId>>,
    my_refund_address: ethereum_support::Address,
    my_success_keypair: KeyPair,
}

impl ComitServer {
    pub fn new(
        event_store: Arc<InMemoryEventStore<TradeId>>,
        my_refund_address: ethereum_support::Address,
        my_success_keypair: KeyPair,
    ) -> Self {
        Self {
            event_store,
            my_refund_address,
            my_success_keypair,
        }
    }

    pub fn listen(self, addr: SocketAddr) -> impl Future<Item = (), Error = io::Error> {
        info!("ComitServer listening at {:?}", addr);
        let socket = TcpListener::bind(&addr).unwrap();

        socket.incoming().for_each(move |connection| {
            let peer_addr = connection.peer_addr();
            let codec = json::JsonFrameCodec::default();
            let swap_handler = MySwapHandler::new(
                self.my_refund_address.clone(),
                self.my_success_keypair.clone(),
                self.event_store.clone(),
            );
            let config = ganp::json_config(swap_handler);
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
    my_refund_address: ethereum_support::Address,
    my_success_keypair: KeyPair,
    event_store: Arc<InMemoryEventStore<TradeId>>,
    runtime: Runtime,
    future_factory: FutureFactory<LedgerServices>,
    ledger_query_service_api_client: C,
    bitcoin_node: Arc<BitcoinRpcApi>,
    ethereum_service: Arc<EthereumService>,
}

impl<C: LedgerQueryServiceApiClient> MySwapHandler<C> {
    pub fn new(
        my_refund_address: ethereum_support::Address,
        my_success_keypair: KeyPair,
        event_store: Arc<InMemoryEventStore<TradeId>>,
        runtime: Runtime,
        future_factory: FutureFactory<LedgerServices>,
        client: C,
        bitcoin_node: Arc<BitcoinRpcApi>,
        ethereum_service: Arc<EthereumService>,
    ) -> Self {
        MySwapHandler {
            my_refund_address,
            my_success_keypair,
            event_store,
            runtime,
            future_factory,
            ledger_query_service_api_client,
            bitcoin_node,
            ethereum_service,
        }
    }
}

impl<C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>>
    SwapRequestHandler<
        rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
        rfc003::AcceptResponse<Bitcoin, Ethereum>,
    > for MySwapHandler<C>
{
    fn handle(
        &mut self,
        request: rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
    ) -> swap::SwapResponse<rfc003::AcceptResponse<Bitcoin, Ethereum>> {
        let alice_refund_address = request.source_ledger_refund_identity.clone().into();

        let bob_success_address = self
            .my_success_keypair
            .public_key()
            .clone()
            .to_p2wpkh_address(request.source_ledger.network())
            .into();

        let twelve_hours = Seconds::new(60 * 60 * 12);

        let order_taken = OrderTaken::<Ethereum, Bitcoin> {
            uid: TradeId::default(),
            contract_secret_lock: request.secret_hash,
            alice_contract_time_lock: request.source_ledger_lock_duration,
            bob_contract_time_lock: twelve_hours,
            alice_refund_address,
            alice_success_address: request.target_ledger_success_identity.into(),
            bob_refund_address: self.my_refund_address.clone(),
            bob_success_address: bob_success_address,
            bob_success_keypair: self.my_success_keypair.clone(),
            buy_amount: request.target_asset,
            sell_amount: request.source_asset,
        };

        match self
            .event_store
            .add_event(order_taken.uid, order_taken.clone())
        {
            Ok(_) => {
                let response = rfc003::AcceptResponse::<Bitcoin, Ethereum> {
                    target_ledger_refund_identity: self.my_refund_address.into(),
                    source_ledger_success_identity: self
                        .my_success_keypair
                        .public_key()
                        .clone()
                        .into(),
                    target_ledger_lock_duration: twelve_hours,
                };
                swap::SwapResponse::Accept(response)
            }
            Err(e) => {
                error!(
                    "Declining trade because of problem with event store {:?}",
                    e
                );
                swap::SwapResponse::Decline
            }
        }
    }
}

impl
    SwapRequestHandler<
        rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>,
        rfc003::AcceptResponse<Ethereum, Bitcoin>,
    > for MySwapHandler
{}
