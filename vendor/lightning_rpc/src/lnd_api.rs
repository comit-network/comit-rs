use futures::Future;
use lightning_rpc_api::LightningRpcApi;
use lnrpc::{client::Lightning, *};
use std::{io, net::SocketAddr};
use tls_api::{self, Certificate, TlsConnector, TlsConnectorBuilder};
use tls_api_native_tls;
use tokio_core::{self, net::TcpStream, reactor::Core};
use tokio_tls_api;
use tower_grpc::{self, codegen::client::http, Request, Response};
use tower_h2::{self, client::Connection};
use tower_http::{self, add_origin};

pub struct TowerLightningClient(
    pub  Lightning<
        tower_http::AddOrigin<
            tower_h2::client::Connection<
                tokio_tls_api::TlsStream<tokio_core::net::TcpStream>,
                tokio_core::reactor::Handle,
                tower_h2::BoxBody,
            >,
        >,
    >,
);

pub struct LndClient {
    core: Core,
    client: TowerLightningClient,
}

#[derive(Debug)]
pub enum Error {
    Tls(tls_api::Error),
    TcpStream(io::Error),
    AddOrigin(add_origin::BuilderError),
    Tower(tower_h2::client::HandshakeError),
}

impl LndClient {
    fn create_tls_connector(
        tls_cert: Certificate,
    ) -> Result<tls_api_native_tls::TlsConnector, Error> {
        let mut connector_builder =
            tls_api_native_tls::TlsConnector::builder().map_err(Error::Tls)?;

        connector_builder
            .add_root_certificate(tls_cert)
            .map_err(Error::Tls)?;

        connector_builder.build().map_err(Error::Tls)
    }

    pub fn new(
        tls_cert: Certificate,
        lnd_addr: SocketAddr,
        origin_uri: http::Uri,
    ) -> Result<Self, Error> {
        let mut core = Core::new().unwrap();
        let reactor = core.handle();

        let connector = Self::create_tls_connector(tls_cert)?;

        let tcp_stream = TcpStream::connect(&lnd_addr, &reactor)
            .map_err(Error::TcpStream)
            .and_then(|socket| {
                // The certificate contains "localhost" and the hostname of the machine lnd
                // is running on at "DNS Name". Hence "localhost" (or the machine hostname for added security)
                // must be passed here
                tokio_tls_api::connect_async(&connector, "localhost", socket).map_err(Error::Tls)
            })
            .and_then(move |socket| {
                // Bind the HTTP/2.0 connection
                Connection::handshake(socket, reactor).map_err(Error::Tower)
            })
            .and_then(move |conn| {
                add_origin::Builder::new()
                    .uri(origin_uri)
                    .build(conn)
                    .map_err(Error::AddOrigin)
            })
            .map(Lightning::new);

        let client = core.run({ tcp_stream })?;

        Ok(LndClient {
            core,
            client: TowerLightningClient(client),
        })
    }
}

impl LightningRpcApi for LndClient {
    type Err = tower_grpc::Error<tower_h2::client::Error>;

    fn add_invoice(&mut self, invoice: Invoice) -> Result<AddInvoiceResponse, Self::Err> {
        let response: Response<AddInvoiceResponse> = self
            .core
            .run({ self.client.0.add_invoice(Request::new(invoice)) })?;
        Ok(response.into_inner())
    }

    fn get_info(&mut self) -> Result<GetInfoResponse, Self::Err> {
        let response: Response<GetInfoResponse> = self
            .core
            .run({ self.client.0.get_info(Request::new(GetInfoRequest {})) })?;
        Ok(response.into_inner())
    }

    fn send_payment(&mut self, send_request: SendRequest) -> Result<SendResponse, Self::Err> {
        let response: Response<SendResponse> = self
            .core
            // TODO: `send_payment` uses streams and may be better to use
            .run({ self.client.0.send_payment_sync(Request::new(send_request)) })?;
        Ok(response.into_inner())
    }
}
