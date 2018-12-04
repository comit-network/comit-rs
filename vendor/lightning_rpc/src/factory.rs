use futures::Future;
use http::Uri;
use lnrpc::client::Lightning;
use macaroon::Macaroon;
use std::{io, net::SocketAddr};
use tls_api::{Certificate, TlsConnector, TlsConnectorBuilder};
use tls_api_native_tls;
use tokio::{net::TcpStream, runtime::TaskExecutor};
use tokio_tls_api;
use tower_grpc;
use tower_h2::{self, client::Connection, BoxBody};
use tower_http::add_origin;
use AddMacaroon;

pub type LndClient = Lightning<
    AddMacaroon<
        add_origin::AddOrigin<
            Connection<tokio_tls_api::TlsStream<TcpStream>, TaskExecutor, BoxBody>,
        >,
    >,
>;

#[derive(Debug)]
pub struct ClientFactory {
    executor: TaskExecutor,
}

impl ClientFactory {
    pub fn new(executor: TaskExecutor) -> Self {
        Self { executor }
    }

    pub fn with_macaroon(
        &self,
        origin_uri: Uri,
        tls_cert: Certificate,
        lnd_addr: SocketAddr,
        macaroon: Macaroon,
    ) -> impl Future<Item = LndClient, Error = Error> {
        let executor = self.executor.clone();

        TcpStream::connect(&lnd_addr)
            .map_err(Error::TcpStream)
            .join(create_tls_connector(tls_cert))
            .and_then(|(tcp_stream, connector)| {
                // The certificate contains "localhost" and the hostname of the machine lnd
                // is running on at "DNS Name". Hence "localhost" (or the machine hostname for
                // added security) must be passed here
                tokio_tls_api::connect_async(&connector, "localhost", tcp_stream)
                    .map_err(Error::Tls)
            })
            .and_then(move |socket| {
                // Bind the HTTP/2.0 connection
                Connection::<_, _, tower_h2::BoxBody>::handshake(socket, executor)
                    .map_err(Error::Tower)
            })
            .and_then(move |conn| {
                add_origin::Builder::new()
                    .uri(origin_uri)
                    .build(conn)
                    .map_err(Error::AddOrigin)
            })
            .map(|conn| AddMacaroon::new(conn, macaroon))
            .map(Lightning::new)
    }
}

fn create_tls_connector(tls_cert: Certificate) -> Result<tls_api_native_tls::TlsConnector, Error> {
    let mut connector_builder = tls_api_native_tls::TlsConnector::builder().map_err(Error::Tls)?;

    connector_builder
        .add_root_certificate(tls_cert)
        .map_err(Error::Tls)?;

    connector_builder.build().map_err(Error::Tls)
}

#[derive(Debug)]
pub enum Error {
    Tls(tls_api::Error),
    TcpStream(io::Error),
    AddOrigin(add_origin::BuilderError),
    Tower(tower_h2::client::HandshakeError),
    Grpc(tower_grpc::Error<tower_h2::client::Error>),
}
