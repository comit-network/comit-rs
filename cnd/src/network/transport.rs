use crate::network::TorTokioTcpConfig;
use libp2p::{
    core::{
        either::EitherError,
        muxing::StreamMuxerBox,
        transport::{boxed::Boxed, timeout::TransportTimeoutError},
        upgrade::{SelectUpgrade, Version},
        UpgradeError,
    },
    dns::{DnsConfig, DnsErr},
    identity,
    mplex::MplexConfig,
    multiaddr::Protocol,
    secio::{SecioConfig, SecioError},
    tcp::TokioTcpConfig,
    yamux, Multiaddr, PeerId, Transport,
};
use std::{collections::HashMap, io, time::Duration};

const PORT: u16 = 9939;

/// Build the Comit libp2p Transport.
pub fn build(keypair: identity::Keypair, listen: Vec<Multiaddr>) -> anyhow::Result<ComitTransport> {
    // It only makes sense to listen on a single address when using Tor.
    if listen.len() == 1 {
        let addr = listen[0].clone();
        if is_onion(addr.clone()) {
            return build_tor_transport(keypair, addr);
        }
    }
    build_comit_transport(keypair)
}

/// Builds a libp2p transport with the following features:
/// - TcpConnection
/// - DNS name resolution
/// - authentication via secio
/// - multiplexing via yamux or mplex
fn build_comit_transport(keypair: identity::Keypair) -> anyhow::Result<ComitTransport> {
    let transport = TokioTcpConfig::new().nodelay(true);
    let transport = DnsConfig::new(transport)?;

    let transport = transport
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(keypair))
        .multiplex(SelectUpgrade::new(
            yamux::Config::default(),
            MplexConfig::new(),
        ))
        .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
        .timeout(Duration::from_secs(20))
        .boxed();

    Ok(transport)
}

/// Builds a libp2p transport with the following features:
/// - TCP connection over the Tor network
/// - DNS name resolution
/// - authentication via secio
/// - multiplexing via yamux or mplex
fn build_tor_transport(
    keypair: identity::Keypair,
    addr: Multiaddr,
) -> anyhow::Result<ComitTransport> {
    let mut map = HashMap::new();
    map.insert(addr, PORT);

    let transport = TorTokioTcpConfig::new().nodelay(true).onion_map(map);
    let transport = DnsConfig::new(transport)?;

    let transport = transport
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(keypair))
        .multiplex(SelectUpgrade::new(
            yamux::Config::default(),
            MplexConfig::new(),
        ))
        .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
        .timeout(Duration::from_secs(20))
        .boxed();

    Ok(transport)
}

pub type ComitTransport = Boxed<
    (PeerId, StreamMuxerBox),
    TransportTimeoutError<
        EitherError<
            EitherError<DnsErr<io::Error>, UpgradeError<SecioError>>,
            UpgradeError<EitherError<io::Error, io::Error>>,
        >,
    >,
>;

// True if `addr` is a Tor onion address v2 or v3.
fn is_onion(mut addr: Multiaddr) -> bool {
    match addr.pop() {
        Some(Protocol::Onion(..)) => true,
        Some(Protocol::Onion3(_)) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_onion_works_positive_v3() {
        let addr = "/onion3/vww6ybal4bd7szmgncyruucpgfkqahzddi37ktceo3ah7ngmcopnpyyd:1234"
            .parse()
            .expect("failed to parse multiaddr");
        assert!(is_onion(addr))
    }

    #[test]
    fn is_onion_works_positive_v2() {
        let addr = "/onion/aaimaq4ygg2iegci:80"
            .parse()
            .expect("failed to parse multiaddr");
        assert!(is_onion(addr))
    }

    #[test]
    fn is_onion_works_negative() {
        let addr = "/ip4/127.0.0.1/tcp/1234"
            .parse()
            .expect("failed to parse multiaddr");
        assert!(!is_onion(addr))
    }
}
