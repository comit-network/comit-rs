use bitcoin_support::{BitcoinQuantity, ToP2wpkhAddress};
use comit_wallet::KeyStore;
use common_types::seconds::Seconds;
use ethereum_support::{EthereumQuantity, ToEthereumAddress};
use event_store::{EventStore, InMemoryEventStore};
use futures::{Future, Stream};
use std::{io, net::SocketAddr, sync::Arc};
use swap_protocols::{
    json_config,
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003,
    wire_types::SwapResponse,
    SwapRequestHandler,
};
use swaps::{bob_events::OrderTaken, common::TradeId};
use tokio::{self, net::TcpListener};
use transport_protocol::{connection::Connection, json};

#[derive(Debug)]
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

    pub fn listen(self, addr: SocketAddr) -> impl Future<Item = (), Error = io::Error> {
        info!("ComitServer listening at {:?}", addr);
        let socket = TcpListener::bind(&addr).unwrap();

        socket.incoming().for_each(move |connection| {
            let peer_addr = connection.peer_addr();
            let codec = json::JsonFrameCodec::default();
            let swap_handler =
                MySwapHandler::new(self.my_keystore.clone(), self.event_store.clone());
            let config = json_config(swap_handler);
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

struct MySwapHandler {
    my_keystore: Arc<KeyStore>,
    event_store: Arc<InMemoryEventStore<TradeId>>,
}

impl MySwapHandler {
    pub fn new(my_keystore: Arc<KeyStore>, event_store: Arc<InMemoryEventStore<TradeId>>) -> Self {
        MySwapHandler {
            my_keystore,
            event_store,
        }
    }
}

const EXTRA_DATA_FOR_TRANSIENT_REDEEM: [u8; 1] = [1];

impl
    SwapRequestHandler<
        rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
        rfc003::AcceptResponse<Bitcoin, Ethereum>,
    > for MySwapHandler
{
    fn handle(
        &mut self,
        request: rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
    ) -> SwapResponse<rfc003::AcceptResponse<Bitcoin, Ethereum>> {
        // TODO: need to remove confusion as bob/my are interchangeable and interchanged. See #297
        // TODO: Prefer "redeem vs refund vs final" terminology than the "success" that may be misleading
        let alice_refund_address = request.source_ledger_refund_identity.clone().into();

        let uid = TradeId::default();

        let bob_success_keypair = self
            .my_keystore
            .get_transient_keypair(&uid.into(), &EXTRA_DATA_FOR_TRANSIENT_REDEEM);
        let bob_success_address = bob_success_keypair
            .public_key()
            .clone()
            .into_p2wpkh_address(request.source_ledger.network())
            .into();
        debug!(
            "Generated transient success address for Bob is {}",
            bob_success_address
        );

        let bob_refund_keypair = self.my_keystore.get_new_internal_keypair();

        let bob_refund_address = bob_refund_keypair.public_key().to_ethereum_address();
        debug!(
            "Generated address for Bob's refund is {}",
            bob_refund_address
        );

        let twelve_hours = Seconds::new(60 * 60 * 12);

        let order_taken = OrderTaken::<Ethereum, Bitcoin> {
            uid,
            contract_secret_lock: request.secret_hash,
            alice_contract_time_lock: request.source_ledger_lock_duration,
            bob_contract_time_lock: twelve_hours,
            alice_refund_address,
            alice_success_address: request.target_ledger_success_identity.into(),
            bob_refund_address: bob_refund_address.clone(),
            bob_success_address,
            bob_success_keypair: bob_success_keypair.clone(),
            buy_amount: request.target_asset,
            sell_amount: request.source_asset,
        };

        match self
            .event_store
            .add_event(order_taken.uid, order_taken.clone())
        {
            Ok(_) => {
                let response = rfc003::AcceptResponse::<Bitcoin, Ethereum> {
                    target_ledger_refund_identity: bob_refund_address.into(),
                    source_ledger_success_identity: bob_success_keypair.public_key().clone().into(),
                    target_ledger_lock_duration: twelve_hours,
                };
                SwapResponse::Accept(response)
            }
            Err(e) => {
                error!(
                    "Declining trade because of problem with event store {:?}",
                    e
                );
                SwapResponse::Decline
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
