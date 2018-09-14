use bitcoin_support::BitcoinQuantity;
use ethereum_support::EthereumQuantity;
use futures::{Future, Stream};
use ganp::{
    self,
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003, SwapRequestHandler,
};
use std::{io, net::SocketAddr};
use tokio::{self, net::TcpListener};
use transport_protocol::{config::Config, connection::Connection, json};

pub struct ComitServer {}

impl ComitServer {
    pub fn listen(addr: SocketAddr) -> impl Future<Item = (), Error = io::Error> {
        info!("ComitServer listening at {:?}", addr);
        let socket = TcpListener::bind(&addr).unwrap();

        socket.incoming().for_each(|connection| {
            let codec = json::JsonFrameCodec::default();
            let config = ganp::json_config(MySwapHandler {});
            let connection = Connection::new(config, codec, connection);
            let (close_future, _client) = connection.start::<json::JsonFrameHandler>();
            tokio::spawn(close_future.map_err(|e| {
                error!("closing connection with client: {:?}", e);
            }));
            Ok(())
        })
    }
}

struct MySwapHandler {}

impl
    SwapRequestHandler<
        rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
        rfc003::AcceptResponse<Bitcoin, Ethereum>,
    > for MySwapHandler
{}

impl
    SwapRequestHandler<
        rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>,
        rfc003::AcceptResponse<Ethereum, Bitcoin>,
    > for MySwapHandler
{}
