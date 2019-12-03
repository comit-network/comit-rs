use libp2p::{
    core::{muxing, upgrade},
    dns, identity, mplex, secio, tcp, yamux, Multiaddr, PeerId, Transport, TransportError,
};
use std::{error, io, time::Duration};

/// Builds a `Transport` that supports the most commonly-used protocols that
/// libp2p supports.
pub fn build_comit_transport(
    keypair: identity::Keypair,
) -> impl Transport<
    Output = (
        PeerId,
        impl muxing::StreamMuxer<
                OutboundSubstream = impl Send,
                Substream = impl Send,
                Error = impl Into<io::Error>,
            > + Send
            + Sync,
    ),
    Error = impl error::Error + Send,
    Listener = impl Send,
    Dial = impl Send,
    ListenerUpgrade = impl Send,
> + Clone {
    build_tcp_ws_secio_mplex_yamux(keypair)
}

/// Builds an implementation of `Transport` that is suitable for usage with the
/// `Swarm`.
///
/// The implementation supports TCP/IP, secio as the encryption layer, and mplex
/// or yamux as the multiplexing layer.
pub fn build_tcp_ws_secio_mplex_yamux(
    keypair: identity::Keypair,
) -> impl Transport<
    Output = (
        PeerId,
        impl muxing::StreamMuxer<
                OutboundSubstream = impl Send,
                Substream = impl Send,
                Error = impl Into<io::Error>,
            > + Send
            + Sync,
    ),
    Error = impl error::Error + Send,
    Listener = impl Send,
    Dial = impl Send,
    ListenerUpgrade = impl Send,
> + Clone {
    ComitTransport::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(secio::SecioConfig::new(keypair))
        .multiplex(upgrade::SelectUpgrade::new(
            yamux::Config::default(),
            mplex::MplexConfig::new(),
        ))
        .map(|(peer, muxer), _| (peer, muxing::StreamMuxerBox::new(muxer)))
        .timeout(Duration::from_secs(20))
}

#[derive(Debug, Clone)]
struct ComitTransport {
    inner: InnerImplementation,
}

type InnerImplementation = dns::DnsConfig<tcp::TcpConfig>;

impl ComitTransport {
    /// Initializes the `ComitTransport`.
    pub fn new() -> ComitTransport {
        let tcp = tcp::TcpConfig::new().nodelay(true);
        let transport = dns::DnsConfig::new(tcp);

        ComitTransport { inner: transport }
    }
}

impl Transport for ComitTransport {
    type Output = <InnerImplementation as Transport>::Output;
    type Error = <InnerImplementation as Transport>::Error;
    type Listener = <InnerImplementation as Transport>::Listener;
    type ListenerUpgrade = <InnerImplementation as Transport>::ListenerUpgrade;
    type Dial = <InnerImplementation as Transport>::Dial;

    fn listen_on(self, addr: Multiaddr) -> Result<Self::Listener, TransportError<Self::Error>> {
        self.inner.listen_on(addr)
    }

    fn dial(self, addr: Multiaddr) -> Result<Self::Dial, TransportError<Self::Error>> {
        self.inner.dial(addr)
    }
}
