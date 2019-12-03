use libp2p::{
    core::{
        muxing::{StreamMuxer, StreamMuxerBox},
        upgrade::{SelectUpgrade, Version},
    },
    dns::DnsConfig,
    identity,
    mplex::MplexConfig,
    secio::SecioConfig,
    tcp::TcpConfig,
    yamux, PeerId, Transport,
};
use std::{error, io, time::Duration};

/// Builds a libp2p transport with the following features:
/// - TcpConnection
/// - DNS name resolution
/// - authentication via secio
/// - multiplexing via yamux or mplex
pub fn build_comit_transport(
    keypair: identity::Keypair,
) -> impl Transport<
    Output = (
        PeerId,
        impl StreamMuxer<
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
    let transport = TcpConfig::new().nodelay(true);
    let transport = DnsConfig::new(transport);

    transport
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(keypair))
        .multiplex(SelectUpgrade::new(
            yamux::Config::default(),
            MplexConfig::new(),
        ))
        .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
        .timeout(Duration::from_secs(20))
}
